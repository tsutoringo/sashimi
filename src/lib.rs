pub mod ast;
pub mod codegen;
pub mod core;
pub mod diagnostic;
pub mod lexer;
pub mod lsp;
pub mod modules;
pub mod parser;
pub mod semantic;
pub mod types;

use std::path::Path;

use ast::{ImportKind, Item, Program};
use diagnostic::CompileError;
use modules::ExternalBindings;

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
    compile_program(program, ExternalBindings::default(), options)
}

pub fn compile_with_path(
    source: &str,
    source_path: &Path,
    options: &CompileOptions,
) -> Result<CompileOutput, CompileError> {
    let tokens = lexer::lex(source)?;
    let program = parser::parse(tokens)?;
    let externals = modules::resolve_imports(&program, source_path)?;
    compile_program(program, externals, options)
}

fn compile_program(
    program: Program,
    externals: ExternalBindings,
    options: &CompileOptions,
) -> Result<CompileOutput, CompileError> {
    let imports = emit_imports(&program);
    let semantic = semantic::analyze_with_externals(program, &options.package_name, externals)?;
    let generated = codegen::generate(&semantic)?;
    let map_name = format!("{}.map", options.output_name);
    let javascript = format!("{imports}{}//# sourceMappingURL={}\n", generated.js, map_name);
    let declarations = format!("{imports}{}", generated.dts);
    let source_map = format!(
        "{{\"version\":3,\"file\":{:?},\"sources\":[{:?}],\"names\":[],\"mappings\":\"\"}}\n",
        options.output_name, options.source_name,
    );
    Ok(CompileOutput {
        javascript,
        declarations,
        source_map,
    })
}

fn emit_imports(program: &Program) -> String {
    let mut output = String::new();
    for item in &program.items {
        let Item::Import(import) = item else { continue };
        let default = import
            .specifiers
            .iter()
            .find(|specifier| specifier.kind == ImportKind::Default);
        let namespace = import
            .specifiers
            .iter()
            .find(|specifier| specifier.kind == ImportKind::Namespace);
        let named = import
            .specifiers
            .iter()
            .filter(|specifier| specifier.kind == ImportKind::Named)
            .collect::<Vec<_>>();

        output.push_str("import ");
        if let Some(default) = default {
            output.push_str(&default.local);
            if namespace.is_some() || !named.is_empty() {
                output.push_str(", ");
            }
        }
        if let Some(namespace) = namespace {
            output.push_str("* as ");
            output.push_str(&namespace.local);
        } else if !named.is_empty() {
            output.push_str("{ ");
            output.push_str(
                &named
                    .iter()
                    .map(|specifier| {
                        let imported = specifier.imported.as_deref().unwrap_or(&specifier.local);
                        if imported == specifier.local {
                            imported.to_string()
                        } else {
                            format!("{imported} as {}", specifier.local)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            output.push_str(" }");
        }
        output.push_str(" from ");
        output.push_str(&format!("{:?};\n", import.source));
    }
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_named_imports() {
        let output = compile(
            "import { values as nums } from \"demo\";\nfn main() { nums(); }",
            &CompileOptions::default(),
        )
        .expect("compile");
        assert!(output.javascript.contains("import { values as nums } from \"demo\";"));
    }
}
