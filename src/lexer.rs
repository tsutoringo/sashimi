use crate::diagnostic::{CompileError, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    Number(String),
    String(String),
    Fn,
    Let,
    Pub,
    Trait,
    Impl,
    For,
    Return,
    Class,
    New,
    True,
    False,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Less,
    Greater,
    Comma,
    Dot,
    Colon,
    Semi,
    Eq,
    Amp,
    Eof,
}

pub fn lex(source: &str) -> Result<Vec<Token>, CompileError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' => {
                tokens.push(tok(TokenKind::LParen, i, i + 1));
                i += 1;
            }
            b')' => {
                tokens.push(tok(TokenKind::RParen, i, i + 1));
                i += 1;
            }
            b'{' => {
                tokens.push(tok(TokenKind::LBrace, i, i + 1));
                i += 1;
            }
            b'}' => {
                tokens.push(tok(TokenKind::RBrace, i, i + 1));
                i += 1;
            }
            b'[' => {
                tokens.push(tok(TokenKind::LBracket, i, i + 1));
                i += 1;
            }
            b']' => {
                tokens.push(tok(TokenKind::RBracket, i, i + 1));
                i += 1;
            }
            b'<' => {
                tokens.push(tok(TokenKind::Less, i, i + 1));
                i += 1;
            }
            b'>' => {
                tokens.push(tok(TokenKind::Greater, i, i + 1));
                i += 1;
            }
            b',' => {
                tokens.push(tok(TokenKind::Comma, i, i + 1));
                i += 1;
            }
            b'.' => {
                tokens.push(tok(TokenKind::Dot, i, i + 1));
                i += 1;
            }
            b':' => {
                tokens.push(tok(TokenKind::Colon, i, i + 1));
                i += 1;
            }
            b';' => {
                tokens.push(tok(TokenKind::Semi, i, i + 1));
                i += 1;
            }
            b'=' => {
                tokens.push(tok(TokenKind::Eq, i, i + 1));
                i += 1;
            }
            b'&' => {
                tokens.push(tok(TokenKind::Amp, i, i + 1));
                i += 1;
            }
            b'"' | b'\'' => {
                let quote = bytes[i];
                i += 1;
                let content_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(CompileError::new("unterminated string literal", Span::new(start, i)));
                }
                let value = source[content_start..i].to_string();
                i += 1;
                tokens.push(tok(TokenKind::String(value), start, i));
            }
            b'0'..=b'9' => {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                tokens.push(tok(TokenKind::Number(source[start..i].to_string()), start, i));
            }
            c if is_ident_start(c) => {
                i += 1;
                while i < bytes.len() && is_ident_continue(bytes[i]) {
                    i += 1;
                }
                let word = &source[start..i];
                let kind = match word {
                    "fn" => TokenKind::Fn,
                    "let" => TokenKind::Let,
                    "pub" => TokenKind::Pub,
                    "trait" => TokenKind::Trait,
                    "impl" => TokenKind::Impl,
                    "for" => TokenKind::For,
                    "return" => TokenKind::Return,
                    "class" => TokenKind::Class,
                    "new" => TokenKind::New,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    _ => TokenKind::Ident(word.to_string()),
                };
                tokens.push(tok(kind, start, i));
            }
            _ => {
                return Err(CompileError::new(
                    format!("unexpected character `{}`", bytes[i] as char),
                    Span::new(i, i + 1),
                ));
            }
        }
    }

    tokens.push(tok(TokenKind::Eof, source.len(), source.len()));
    Ok(tokens)
}

fn tok(kind: TokenKind, start: usize, end: usize) -> Token {
    Token { kind, span: Span::new(start, end) }
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$'
}

fn is_ident_continue(c: u8) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}
