pub mod ast;
pub mod codegen;
pub mod core;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod types;

use diagnostic::CompileError;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub package_name: String,
    pub source_name: String,
    pub output_name: String,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            package_name: "main".to_string(),
            source_name: "input.sashimi".to_string(),
            output_name: "input.js".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub javascript: String,
    pub declarations: String,
    pub source_map: String,
}

pub fn compile(source: &str, options: &CompileOptions) -> Result<CompileOutput, CompileError> {
    let tokens = lexer::lex(source)?;
    let program = parser::parse(tokens)?;
    let semantic = semantic::analyze(program, &options.package_name)?;
    let generated = codegen::generate(&semantic)?;
    let map_name = format!("{}.map", options.output_name);
    let javascript = format!("{}//# sourceMappingURL={}\n", generated.js, map_name);
    let source_map = format!(
        "{{\"version\":3,\"file\":{:?},\"sources\":[{:?}],\"names\":[],\"mappings\":\"\"}}\n",
        options.output_name, options.source_name,
    );
    Ok(CompileOutput {
        javascript,
        declarations: generated.dts,
        source_map,
    })
}
