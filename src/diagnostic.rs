use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
    pub span: Span,
}

impl CompileError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    pub fn render(&self, source: &str, file: &str) -> String {
        let offset = self.span.start.min(source.len());
        let before = &source[..offset];
        let line = before.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = before
            .rsplit_once('\n')
            .map_or(before.len() + 1, |(_, tail)| tail.len() + 1);
        format!("{file}:{line}:{column}: error: {}", self.message)
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CompileError {}
