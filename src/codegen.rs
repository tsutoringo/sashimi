use std::collections::HashMap;

use crate::{
    ast::{Expr, Function, Item, Stmt},
    core::Lowering,
    diagnostic::{CompileError, Span},
    semantic::{infer_expr, SemanticProgram},
    types::Type,
};

pub struct Generated {
    pub js: String,
    pub dts: String,
}

pub fn generate(semantic: &SemanticProgram) -> Result<Generated, CompileError> {
    let mut js = String::new();
    let mut dts = String::new();

    emit_symbol_declarations(semantic, &mut js);

    for item in &semantic.program.items {
        match item {
            Item::Trait(_) => {}
            Item::Impl(imp) => emit_impl(imp, semantic, &mut js)?,
            Item::Class(class) => {
                if class.public {
                    js.push_str("export ");
                }
                js.push_str(&format!("class {} {{}}\n", class.name));
                if class.public {
                    dts.push_str(&format!("export declare class {} {{}}\n", class.name));
                }
            }
            Item::Function(function) => {
                emit_function(function, semantic, &mut js)?;
                if function.public {
                    emit_dts_function(function, &mut dts);
                }
            }
        }
    }

    Ok(Generated { js, dts })
}

fn emit_symbol_declarations(semantic: &SemanticProgram, js: &mut String) {
    for imp in &semantic.impls {
        for method in &imp.source.methods {
            let symbol = symbol_ident(&imp.source.trait_name, &method.name);
            let key = format!(
                "sashimi:{}::{}.{}",
                semantic.package_name, imp.source.trait_name, method.name
            );
            js.push_str(&format!("const {symbol} = Symbol.for({key:?});\n"));
        }
    }
    if !semantic.impls.is_empty() {
        js.push('\n');
    }
}

fn emit_impl(
    imp: &crate::ast::ImplDecl,
    semantic: &SemanticProgram,
    js: &mut String,
) -> Result<(), CompileError> {
    for method in &imp.methods {
        let symbol = symbol_ident(&imp.trait_name, &method.name);
        js.push_str(&format!("{}.prototype[{symbol}] = function(", imp.target.name));
        let params = method
            .params
            .iter()
            .filter(|p| !p.receiver)
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        js.push_str(&params);
        js.push_str(") {\n");
        let receiver = Type::from_ref(&imp.target, &semantic.local_classes);
        let mut env = HashMap::new();
        for param in &method.params {
            if param.receiver {
                env.insert(param.name.clone(), receiver.clone());
            } else if let Some(ty) = &param.ty {
                env.insert(param.name.clone(), Type::from_ref(ty, &semantic.local_classes));
            }
        }
        emit_body(&method.body, semantic, &mut env, js, 1)?;
        js.push_str("};\n\n");
    }
    Ok(())
}

fn emit_function(
    function: &Function,
    semantic: &SemanticProgram,
    js: &mut String,
) -> Result<(), CompileError> {
    if function.public {
        js.push_str("export ");
    }
    js.push_str(&format!("function {}(", function.name));
    js.push_str(
        &function
            .params
            .iter()
            .filter(|p| !p.receiver)
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    js.push_str(") {\n");
    let mut env = HashMap::new();
    for param in &function.params {
        if let Some(ty) = &param.ty {
            env.insert(param.name.clone(), Type::from_ref(ty, &semantic.local_classes));
        }
    }
    emit_body(&function.body, semantic, &mut env, js, 1)?;
    js.push_str("}\n\n");
    Ok(())
}

fn emit_body(
    statements: &[Stmt],
    semantic: &SemanticProgram,
    env: &mut HashMap<String, Type>,
    js: &mut String,
    indent: usize,
) -> Result<(), CompileError> {
    for statement in statements {
        js.push_str(&"    ".repeat(indent));
        match statement {
            Stmt::Let { name, value } => {
                let info = infer_expr(value, env, semantic)?;
                js.push_str(&format!("const {name} = {}", emit_expr(value, semantic, env)?));
                env.insert(name.clone(), info.ty);
            }
            Stmt::Expr(expr) => js.push_str(&emit_expr(expr, semantic, env)?),
            Stmt::Return(Some(expr)) => {
                js.push_str(&format!("return {}", emit_expr(expr, semantic, env)?));
            }
            Stmt::Return(None) => js.push_str("return"),
        }
        js.push_str(";\n");
    }
    Ok(())
}

fn emit_expr(
    expr: &Expr,
    semantic: &SemanticProgram,
    env: &HashMap<String, Type>,
) -> Result<String, CompileError> {
    match expr {
        Expr::Number(value) => Ok(value.clone()),
        Expr::String(value) => Ok(format!("{value:?}")),
        Expr::Bool(value) => Ok(value.to_string()),
        Expr::Array(items) => Ok(format!(
            "[{}]",
            items
                .iter()
                .map(|item| emit_expr(item, semantic, env))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        Expr::Ident(name) if name == "self" => Ok("this".to_string()),
        Expr::Ident(name) => Ok(name.clone()),
        Expr::New { class_name, args } => Ok(format!(
            "new {class_name}({})",
            emit_args(args, semantic, env)?
        )),
        Expr::Member { object, property } => Ok(format!(
            "{}.{}",
            emit_expr(object, semantic, env)?,
            property
        )),
        Expr::Call { callee, args } => Ok(format!(
            "{}({})",
            emit_expr(callee, semantic, env)?,
            emit_args(args, semantic, env)?
        )),
        Expr::MethodCall { receiver, method, args } => {
            let receiver_js = emit_expr(receiver, semantic, env)?;
            let info = infer_expr(expr, env, semantic)?;
            match info.lowering {
                Some(Lowering::Property(property)) if args.is_empty() => {
                    Ok(format!("{receiver_js}.{property}"))
                }
                Some(Lowering::Property(_)) => Err(CompileError::new(
                    "property intrinsic does not accept arguments",
                    Span::new(0, 0),
                )),
                Some(Lowering::Symbol { trait_name, method, .. }) => Ok(format!(
                    "{receiver_js}[{}]({})",
                    symbol_ident(&trait_name, &method),
                    emit_args(args, semantic, env)?
                )),
                None => Ok(format!(
                    "{receiver_js}.{method}({})",
                    emit_args(args, semantic, env)?
                )),
            }
        }
    }
}

fn emit_args(
    args: &[Expr],
    semantic: &SemanticProgram,
    env: &HashMap<String, Type>,
) -> Result<String, CompileError> {
    Ok(args
        .iter()
        .map(|arg| emit_expr(arg, semantic, env))
        .collect::<Result<Vec<_>, _>>()?
        .join(", "))
}

fn emit_dts_function(function: &Function, dts: &mut String) {
    let params = function
        .params
        .iter()
        .filter(|p| !p.receiver)
        .map(|param| {
            format!(
                "{}: {}",
                param.name,
                param
                    .ty
                    .as_ref()
                    .map_or("unknown".to_string(), |ty| ty.to_typescript())
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret = function
        .return_type
        .as_ref()
        .map_or("void".to_string(), |ty| ty.to_typescript());
    dts.push_str(&format!(
        "export declare function {}({params}): {ret};\n",
        function.name
    ));
}

fn symbol_ident(trait_name: &str, method: &str) -> String {
    format!(
        "__sashimi_trait_{}_{}",
        sanitize(trait_name),
        sanitize(method)
    )
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
