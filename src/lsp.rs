use std::{
    collections::{BTreeSet, HashMap},
    io::{self, BufRead, Write},
    path::PathBuf,
};

use serde_json::{Value, json};

use crate::{CompileOptions, compile, compile_with_path};

pub fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut documents = HashMap::<String, String>::new();
    let mut shutdown = false;

    while let Some(message) = read_message(&mut reader)? {
        let method = message.get("method").and_then(Value::as_str).unwrap_or_default();
        let id = message.get("id").cloned();

        match method {
            "initialize" => {
                respond(
                    &mut writer,
                    id,
                    json!({
                        "capabilities": {
                            "textDocumentSync": {
                                "openClose": true,
                                "change": 1
                            },
                            "completionProvider": {
                                "triggerCharacters": [".", ":"]
                            },
                            "hoverProvider": true,
                            "documentSymbolProvider": true
                        },
                        "serverInfo": {
                            "name": "sashimi-lsp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }),
                )?;
            }
            "initialized" => {}
            "shutdown" => {
                shutdown = true;
                respond(&mut writer, id, Value::Null)?;
            }
            "exit" => break,
            "textDocument/didOpen" => {
                if let Some(document) = message.pointer("/params/textDocument") {
                    if let (Some(uri), Some(text)) = (
                        document.get("uri").and_then(Value::as_str),
                        document.get("text").and_then(Value::as_str),
                    ) {
                        documents.insert(uri.to_string(), text.to_string());
                        publish_diagnostics(&mut writer, uri, text)?;
                    }
                }
            }
            "textDocument/didChange" => {
                let uri = message
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let text = message
                    .pointer("/params/contentChanges/0/text")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let (Some(uri), Some(text)) = (uri, text) {
                    documents.insert(uri.clone(), text.clone());
                    publish_diagnostics(&mut writer, &uri, &text)?;
                }
            }
            "textDocument/completion" => {
                let uri = message.pointer("/params/textDocument/uri").and_then(Value::as_str);
                let source = uri.and_then(|uri| documents.get(uri)).map_or("", String::as_str);
                respond(&mut writer, id, Value::Array(completion_items(source)))?;
            }
            "textDocument/hover" => {
                let uri = message.pointer("/params/textDocument/uri").and_then(Value::as_str);
                let line = message.pointer("/params/position/line").and_then(Value::as_u64);
                let character = message.pointer("/params/position/character").and_then(Value::as_u64);
                let result = uri
                    .and_then(|uri| documents.get(uri))
                    .zip(line.zip(character))
                    .and_then(|(source, (line, character))| hover(source, line as usize, character as usize))
                    .unwrap_or(Value::Null);
                respond(&mut writer, id, result)?;
            }
            "textDocument/documentSymbol" => {
                let uri = message.pointer("/params/textDocument/uri").and_then(Value::as_str);
                let source = uri.and_then(|uri| documents.get(uri)).map_or("", String::as_str);
                respond(&mut writer, id, Value::Array(document_symbols(source)))?;
            }
            _ if id.is_some() => {
                error_response(&mut writer, id, -32601, "method not found")?;
            }
            _ => {}
        }

        if shutdown && method == "exit" {
            break;
        }
    }

    Ok(())
}

fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).map_err(|error| error.to_string())?;
        if read == 0 {
            return Ok(None);
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "invalid Content-Length header".to_string())?,
            );
        }
    }

    let length = content_length.ok_or_else(|| "missing Content-Length header".to_string())?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|error| error.to_string())?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn write_message(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len()).map_err(|error| error.to_string())?;
    writer.write_all(&body).map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn respond(writer: &mut impl Write, id: Option<Value>, result: Value) -> Result<(), String> {
    let Some(id) = id else { return Ok(()) };
    write_message(writer, &json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn error_response(writer: &mut impl Write, id: Option<Value>, code: i64, message: &str) -> Result<(), String> {
    let Some(id) = id else { return Ok(()) };
    write_message(
        writer,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    )
}

fn publish_diagnostics(writer: &mut impl Write, uri: &str, source: &str) -> Result<(), String> {
    let options = CompileOptions {
        package_name: "lsp".to_string(),
        source_name: uri.to_string(),
        output_name: "lsp.js".to_string(),
    };
    let result = file_uri_to_path(uri).map_or_else(
        || compile(source, &options),
        |path| compile_with_path(source, &path, &options),
    );

    let diagnostics = match result {
        Ok(_) => Vec::new(),
        Err(error) => {
            let start = offset_to_position(source, error.span.start);
            let end_offset = error.span.end.max(error.span.start.saturating_add(1));
            let end = offset_to_position(source, end_offset);
            vec![json!({
                "range": { "start": start, "end": end },
                "severity": 1,
                "source": "sashimi",
                "message": error.message
            })]
        }
    };

    write_message(
        writer,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "diagnostics": diagnostics }
        }),
    )
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let decoded = percent_decode(encoded)?;
    #[cfg(windows)]
    let decoded = decoded.strip_prefix('/').unwrap_or(&decoded).to_string();
    Some(PathBuf::from(decoded))
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            output.push(hex(high)? * 16 + hex(low)?);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn offset_to_position(source: &str, offset: usize) -> Value {
    let offset = offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let tail = prefix.rsplit_once('\n').map_or(prefix, |(_, tail)| tail);
    json!({ "line": line, "character": tail.encode_utf16().count() })
}

pub fn completion_items(source: &str) -> Vec<Value> {
    let mut names = BTreeSet::new();
    for keyword in [
        "fn", "pub", "let", "return", "trait", "impl", "for", "class", "new", "import", "from", "as", "true",
        "false",
    ] {
        names.insert((keyword.to_string(), 14, "Sashimi keyword".to_string()));
    }
    for ty in ["number", "string", "boolean", "Array", "Map", "Set", "Iterator"] {
        names.insert((ty.to_string(), 7, "Sashimi type".to_string()));
    }
    for method in [
        "len", "iter", "next", "map", "filter", "take", "skip", "enumerate", "chain", "zip", "inspect", "flat_map",
        "flatten", "collect", "count", "nth", "last", "find", "position", "any", "all", "fold", "reduce", "sum",
        "product", "min", "max", "for_each",
    ] {
        names.insert((method.to_string(), 2, "Prelude trait method".to_string()));
    }

    for (kind, name) in declared_names(source) {
        names.insert((name, kind, "Declared in this document".to_string()));
    }

    names
        .into_iter()
        .map(|(label, kind, detail)| json!({ "label": label, "kind": kind, "detail": detail }))
        .collect()
}

fn hover(source: &str, line: usize, character: usize) -> Option<Value> {
    let word = word_at(source, line, character)?;
    let markdown = match word.as_str() {
        "Len" | "len" => "`Len::len` returns the size of a core collection.",
        "IntoIterator" | "iter" => "`IntoIterator::iter` creates a fresh lazy Sashimi `Iterator`.",
        "Iterator" => "`Iterator<T>` is Sashimi's lazy, one-shot iterator type.",
        "map" => "`Iterator::map(mapper)` lazily transforms each item.",
        "filter" => "`Iterator::filter(predicate)` lazily keeps matching items.",
        "collect" => "`Iterator::collect()` consumes the iterator into an `Array`.",
        "fold" => "`Iterator::fold(initial, folder)` consumes the iterator into one value.",
        "Array" => "JavaScript `Array<T>`. Core provides `Len` and `IntoIterator`.",
        "Map" => "JavaScript `Map<K, V>`. `iter()` yields `[K, V]` entries.",
        "Set" => "JavaScript `Set<T>`. Core currently provides `Len`.",
        "trait" => "Declares shared behavior resolved statically by the Sashimi compiler.",
        "impl" => "Implements a trait for a local type. Foreign-type impls are reserved for trusted core.",
        "import" => "Imports a JavaScript module. Sashimi reads matching TypeScript declarations when available.",
        _ => return None,
    };
    Some(json!({ "contents": { "kind": "markdown", "value": markdown } }))
}

fn word_at(source: &str, line: usize, character: usize) -> Option<String> {
    let line_text = source.lines().nth(line)?;
    let mut utf16 = 0;
    let mut byte = line_text.len();
    for (index, ch) in line_text.char_indices() {
        if utf16 >= character {
            byte = index;
            break;
        }
        utf16 += ch.len_utf16();
    }
    let bytes = line_text.as_bytes();
    let mut start = byte.min(bytes.len());
    let mut end = start;
    while start > 0 && is_ident(bytes[start - 1]) {
        start -= 1;
    }
    while end < bytes.len() && is_ident(bytes[end]) {
        end += 1;
    }
    (start < end).then(|| line_text[start..end].to_string())
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn declared_names(source: &str) -> Vec<(u8, String)> {
    let tokens = source
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    for pair in tokens.windows(2) {
        let kind = match pair[0] {
            "fn" => 3,
            "class" => 7,
            "trait" => 8,
            "let" => 6,
            _ => continue,
        };
        result.push((kind, pair[1].to_string()));
    }
    result
}

fn document_symbols(source: &str) -> Vec<Value> {
    declared_names(source)
        .into_iter()
        .map(|(kind, name)| {
            json!({
                "name": name,
                "kind": kind,
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": source.lines().count().saturating_sub(1), "character": 0 }
                },
                "selectionRange": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 0 }
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_contains_prelude_and_local_names() {
        let items = completion_items("class User {}\nfn greet() {}");
        let labels = items
            .iter()
            .filter_map(|item| item.get("label").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(labels.contains(&"iter"));
        assert!(labels.contains(&"User"));
        assert!(labels.contains(&"greet"));
    }

    #[test]
    fn hover_describes_iterator() {
        let value = hover("Iterator", 0, 4).expect("hover should exist");
        assert!(value.to_string().contains("lazy"));
    }

    #[test]
    fn decodes_file_uri_paths() {
        let path = file_uri_to_path("file:///tmp/sashimi%20project/main.sashimi").expect("file uri");
        assert!(path.to_string_lossy().contains("sashimi project"));
    }
}
