use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde_json::Value;

use crate::{
    ast::{ImportKind, Item, Program},
    diagnostic::{CompileError, Span},
    types::Type,
};

#[derive(Debug, Clone, Default)]
pub struct ExternalBindings {
    pub values: HashMap<String, Type>,
}

pub fn resolve_imports(program: &Program, source_path: &Path) -> Result<ExternalBindings, CompileError> {
    let mut bindings = ExternalBindings::default();

    for item in &program.items {
        let Item::Import(import) = item else { continue };
        let declarations = resolve_declaration_path(source_path, &import.source)
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|source| parse_declarations(&source))
            .unwrap_or_default();

        for specifier in &import.specifiers {
            let ty = match specifier.kind {
                ImportKind::Namespace => Type::Namespace(declarations.clone()),
                ImportKind::Named | ImportKind::Default => specifier
                    .imported
                    .as_ref()
                    .and_then(|name| declarations.get(name))
                    .cloned()
                    .unwrap_or(Type::Unknown),
            };
            bindings.values.insert(specifier.local.clone(), ty);
        }
    }

    Ok(bindings)
}

pub fn resolve_declaration_path(source_path: &Path, specifier: &str) -> Option<PathBuf> {
    if specifier.starts_with('.') || specifier.starts_with('/') {
        return resolve_relative(source_path, specifier);
    }
    resolve_package(source_path, specifier)
}

fn resolve_relative(source_path: &Path, specifier: &str) -> Option<PathBuf> {
    let base = source_path.parent().unwrap_or_else(|| Path::new("."));
    let requested = if specifier.starts_with('/') {
        PathBuf::from(specifier)
    } else {
        base.join(specifier)
    };

    declaration_candidates(&requested)
        .into_iter()
        .find(|path| path.is_file())
}

fn declaration_candidates(requested: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let text = requested.to_string_lossy();
    if text.ends_with(".d.ts") {
        candidates.push(requested.to_path_buf());
        return candidates;
    }

    if requested.extension().is_some() {
        candidates.push(requested.with_extension("d.ts"));
    } else {
        candidates.push(PathBuf::from(format!("{text}.d.ts")));
    }
    candidates.push(requested.join("index.d.ts"));
    candidates
}

fn resolve_package(source_path: &Path, specifier: &str) -> Option<PathBuf> {
    let (package_name, subpath) = split_package_specifier(specifier)?;
    let mut current = source_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    loop {
        let package_root = current.join("node_modules").join(&package_name);
        if package_root.is_dir() {
            return resolve_package_types(&package_root, subpath.as_deref());
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn split_package_specifier(specifier: &str) -> Option<(String, Option<String>)> {
    let parts = specifier.split('/').collect::<Vec<_>>();
    if parts.is_empty() || parts[0].is_empty() {
        return None;
    }
    if specifier.starts_with('@') {
        if parts.len() < 2 {
            return None;
        }
        let package = format!("{}/{}", parts[0], parts[1]);
        let subpath = (parts.len() > 2).then(|| parts[2..].join("/"));
        Some((package, subpath))
    } else {
        let subpath = (parts.len() > 1).then(|| parts[1..].join("/"));
        Some((parts[0].to_string(), subpath))
    }
}

fn resolve_package_types(package_root: &Path, subpath: Option<&str>) -> Option<PathBuf> {
    let package_json_path = package_root.join("package.json");
    let package_json = fs::read_to_string(&package_json_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());

    if let Some(subpath) = subpath {
        if let Some(json) = &package_json {
            let key = format!("./{subpath}");
            if let Some(target) = json
                .get("exports")
                .and_then(|exports| exports.get(&key))
                .and_then(types_export_target)
            {
                let path = package_root.join(target.trim_start_matches("./"));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        let requested = package_root.join(subpath);
        return declaration_candidates(&requested)
            .into_iter()
            .find(|path| path.is_file());
    }

    if let Some(json) = &package_json {
        for key in ["types", "typings"] {
            if let Some(target) = json.get(key).and_then(Value::as_str) {
                let path = package_root.join(target.trim_start_matches("./"));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        if let Some(target) = json
            .get("exports")
            .and_then(|exports| exports.get(".").or_else(|| Some(exports)))
            .and_then(types_export_target)
        {
            let path = package_root.join(target.trim_start_matches("./"));
            if path.is_file() {
                return Some(path);
            }
        }
    }

    let index = package_root.join("index.d.ts");
    index.is_file().then_some(index)
}

fn types_export_target(value: &Value) -> Option<&str> {
    match value {
        Value::Object(map) => {
            if let Some(target) = map.get("types").and_then(Value::as_str) {
                return Some(target);
            }
            for key in ["import", "default", "node"] {
                if let Some(target) = map.get(key).and_then(types_export_target) {
                    return Some(target);
                }
            }
            None
        }
        _ => None,
    }
}

pub fn parse_declarations(source: &str) -> HashMap<String, Type> {
    let mut exports = HashMap::new();

    let function = Regex::new(
        r"(?m)export\s+(?:default\s+)?(?:declare\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*\([^)]*\)\s*:\s*([^;{]+)",
    )
    .expect("valid regex");
    for captures in function.captures_iter(source) {
        let name = captures[1].to_string();
        let return_type = parse_ts_type(captures[2].trim());
        exports.insert(name, Type::Function(Box::new(return_type)));
    }

    let class = Regex::new(r"(?m)export\s+(?:default\s+)?(?:declare\s+)?class\s+([A-Za-z_$][A-Za-z0-9_$]*)")
        .expect("valid regex");
    for captures in class.captures_iter(source) {
        exports.insert(captures[1].to_string(), Type::External(captures[1].to_string()));
    }

    let interface =
        Regex::new(r"(?m)export\s+(?:declare\s+)?interface\s+([A-Za-z_$][A-Za-z0-9_$]*)").expect("valid regex");
    for captures in interface.captures_iter(source) {
        exports.insert(captures[1].to_string(), Type::External(captures[1].to_string()));
    }

    let constant =
        Regex::new(r"(?m)export\s+(?:declare\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*([^;]+)")
            .expect("valid regex");
    for captures in constant.captures_iter(source) {
        exports.insert(captures[1].to_string(), parse_ts_type(captures[2].trim()));
    }

    // Common declaration shape: `export default Foo;` after a named declaration.
    let default_ref = Regex::new(r"(?m)export\s+default\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*;").expect("valid regex");
    if let Some(captures) = default_ref.captures(source) {
        if let Some(ty) = exports.get(&captures[1]).cloned() {
            exports.insert("default".to_string(), ty);
        }
    }

    exports
}

fn parse_ts_type(source: &str) -> Type {
    let source = source.trim().trim_matches('(').trim_matches(')').trim();
    if source.contains('|') || source.contains('&') || source.contains("=>") {
        return Type::Unknown;
    }
    if let Some(element) = source.strip_suffix("[]") {
        return Type::Array(Box::new(parse_ts_type(element)));
    }
    if source.starts_with('[') && source.ends_with(']') {
        return Type::Tuple(
            split_top_level(&source[1..source.len() - 1])
                .into_iter()
                .map(parse_ts_type)
                .collect(),
        );
    }
    if let Some(open) = source.find('<') {
        if source.ends_with('>') {
            let name = source[..open].trim();
            let args = split_top_level(&source[open + 1..source.len() - 1])
                .into_iter()
                .map(parse_ts_type)
                .collect::<Vec<_>>();
            return match name {
                "Array" | "ReadonlyArray" => Type::Array(Box::new(args.into_iter().next().unwrap_or(Type::Unknown))),
                "Map" | "ReadonlyMap" => Type::Map(
                    Box::new(args.first().cloned().unwrap_or(Type::Unknown)),
                    Box::new(args.get(1).cloned().unwrap_or(Type::Unknown)),
                ),
                "Set" | "ReadonlySet" => Type::Set(Box::new(args.into_iter().next().unwrap_or(Type::Unknown))),
                "Iterator" | "IterableIterator" | "SashimiIterator" => {
                    Type::Iterator(Box::new(args.into_iter().next().unwrap_or(Type::Unknown)))
                }
                other => Type::External(other.to_string()),
            };
        }
    }

    match source {
        "number" => Type::Number,
        "string" => Type::String,
        "boolean" => Type::Boolean,
        "unknown" | "any" | "void" | "undefined" | "null" | "never" => Type::Unknown,
        literal if literal.starts_with('"') || literal.starts_with('\'') => Type::String,
        literal if literal.parse::<f64>().is_ok() => Type::Number,
        other => Type::External(other.to_string()),
    }
}

fn split_top_level(source: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in source.char_indices() {
        match ch {
            '<' | '[' | '(' => depth += 1,
            '>' | ']' | ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                result.push(source[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < source.len() {
        result.push(source[start..].trim());
    }
    result
}

pub fn module_error(message: impl Into<String>) -> CompileError {
    CompileError::new(message, Span::new(0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_dts_exports() {
        let exports = parse_declarations(
            "export declare function values(): Array<number>;\nexport declare const label: string;\nexport declare class Box {}",
        );
        assert_eq!(exports.get("label"), Some(&Type::String));
        assert_eq!(
            exports.get("values"),
            Some(&Type::Function(Box::new(Type::Array(Box::new(Type::Number)))))
        );
        assert!(matches!(exports.get("Box"), Some(Type::External(name)) if name == "Box"));
    }
}
