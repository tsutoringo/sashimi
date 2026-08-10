use std::collections::{HashMap, HashSet};

use crate::{
    ast::{Expr, Function, ImplDecl, Item, Program, Stmt, TraitDecl, TypeRef},
    core::{self, Lowering},
    diagnostic::{CompileError, Span},
    types::Type,
};

#[derive(Debug, Clone)]
pub struct SemanticProgram {
    pub program: Program,
    pub traits: HashMap<String, TraitDecl>,
    pub impls: Vec<ResolvedImpl>,
    pub local_classes: HashSet<String>,
    pub package_name: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedImpl {
    pub source: ImplDecl,
    pub target_type: Type,
}

#[derive(Debug, Clone)]
pub struct ExprInfo {
    pub ty: Type,
    pub lowering: Option<Lowering>,
}

pub fn analyze(program: Program, package_name: &str) -> Result<SemanticProgram, CompileError> {
    let local_classes = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Class(class) => Some(class.name.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    let mut traits = HashMap::new();
    for name in core::prelude_traits() {
        traits.insert(
            (*name).to_string(),
            TraitDecl {
                public: true,
                name: (*name).to_string(),
                methods: vec![crate::ast::TraitMethod {
                    name: "len".to_string(),
                    params: vec![crate::ast::Param {
                        name: "self".to_string(),
                        ty: None,
                        receiver: true,
                    }],
                    return_type: Some(TypeRef::simple("number")),
                }],
            },
        );
    }

    for item in &program.items {
        if let Item::Trait(trait_decl) = item {
            if traits.contains_key(&trait_decl.name) {
                return Err(CompileError::new(
                    format!(
                        "trait `{}` is already defined or provided by the prelude",
                        trait_decl.name
                    ),
                    Span::new(0, 0),
                ));
            }
            traits.insert(trait_decl.name.clone(), trait_decl.clone());
        }
    }

    let mut impls = Vec::new();
    let mut impl_keys = HashSet::new();
    for item in &program.items {
        let Item::Impl(imp) = item else { continue };
        validate_impl(imp, &traits, &local_classes)?;
        let key = (imp.trait_name.clone(), imp.target.to_typescript());
        if !impl_keys.insert(key) {
            return Err(CompileError::new(
                format!(
                    "conflicting implementations of trait `{}` for type `{}`",
                    imp.trait_name,
                    imp.target.to_typescript()
                ),
                Span::new(0, 0),
            ));
        }
        impls.push(ResolvedImpl {
            source: imp.clone(),
            target_type: Type::from_ref(&imp.target, &local_classes),
        });
    }

    let semantic = SemanticProgram {
        program,
        traits,
        impls,
        local_classes,
        package_name: package_name.to_string(),
    };
    validate_bodies(&semantic)?;
    Ok(semantic)
}

fn validate_impl(
    imp: &ImplDecl,
    traits: &HashMap<String, TraitDecl>,
    local_classes: &HashSet<String>,
) -> Result<(), CompileError> {
    let Some(trait_decl) = traits.get(&imp.trait_name) else {
        return Err(CompileError::new(
            format!("unknown trait `{}`", imp.trait_name),
            Span::new(0, 0),
        ));
    };

    if core::is_foreign_builtin(&imp.target.name) || !local_classes.contains(&imp.target.name) {
        return Err(CompileError::new(
            format!(
                "cannot implement trait `{}` for foreign type `{}`; only compiler-trusted core may implement traits for foreign types",
                imp.trait_name,
                imp.target.to_typescript()
            ),
            Span::new(0, 0),
        ));
    }

    for method in &imp.methods {
        if !trait_decl.methods.iter().any(|candidate| candidate.name == method.name) {
            return Err(CompileError::new(
                format!("method `{}` is not a member of trait `{}`", method.name, imp.trait_name),
                Span::new(0, 0),
            ));
        }
    }
    for required in &trait_decl.methods {
        if !imp.methods.iter().any(|method| method.name == required.name) {
            return Err(CompileError::new(
                format!(
                    "implementation of `{}` is missing method `{}`",
                    imp.trait_name, required.name
                ),
                Span::new(0, 0),
            ));
        }
    }
    Ok(())
}

fn validate_bodies(semantic: &SemanticProgram) -> Result<(), CompileError> {
    for item in &semantic.program.items {
        match item {
            Item::Function(function) => check_function(function, semantic)?,
            Item::Impl(imp) => {
                let receiver = Type::from_ref(&imp.target, &semantic.local_classes);
                for method in &imp.methods {
                    check_function_with_receiver(method, semantic, Some(receiver.clone()))?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_function(function: &Function, semantic: &SemanticProgram) -> Result<(), CompileError> {
    check_function_with_receiver(function, semantic, None)
}

fn check_function_with_receiver(
    function: &Function,
    semantic: &SemanticProgram,
    receiver: Option<Type>,
) -> Result<(), CompileError> {
    let mut env = HashMap::new();
    for param in &function.params {
        if param.receiver {
            if let Some(receiver) = &receiver {
                env.insert(param.name.clone(), receiver.clone());
            }
        } else if let Some(ty) = &param.ty {
            env.insert(param.name.clone(), Type::from_ref(ty, &semantic.local_classes));
        }
    }
    for statement in &function.body {
        check_stmt(statement, &mut env, semantic)?;
    }
    Ok(())
}

fn check_stmt(stmt: &Stmt, env: &mut HashMap<String, Type>, semantic: &SemanticProgram) -> Result<(), CompileError> {
    match stmt {
        Stmt::Let { name, value } => {
            let info = infer_expr(value, env, semantic)?;
            env.insert(name.clone(), info.ty);
        }
        Stmt::Expr(expr) => {
            infer_expr(expr, env, semantic)?;
        }
        Stmt::Return(Some(expr)) => {
            infer_expr(expr, env, semantic)?;
        }
        Stmt::Return(None) => {}
    }
    Ok(())
}

pub fn infer_expr(
    expr: &Expr,
    env: &HashMap<String, Type>,
    semantic: &SemanticProgram,
) -> Result<ExprInfo, CompileError> {
    match expr {
        Expr::Number(_) => Ok(info(Type::Number)),
        Expr::String(_) => Ok(info(Type::String)),
        Expr::Bool(_) => Ok(info(Type::Boolean)),
        Expr::Array(values) => {
            let element = values
                .first()
                .map(|value| infer_expr(value, env, semantic).map(|x| x.ty))
                .transpose()?
                .unwrap_or(Type::Unknown);
            Ok(info(Type::Array(Box::new(element))))
        }
        Expr::Ident(name) => Ok(info(env.get(name).cloned().unwrap_or(Type::External(name.clone())))),
        Expr::New { class_name, args } => {
            for arg in args {
                infer_expr(arg, env, semantic)?;
            }
            if semantic.local_classes.contains(class_name) {
                Ok(info(Type::LocalClass(class_name.clone())))
            } else {
                Ok(info(Type::External(class_name.clone())))
            }
        }
        Expr::Member { object, .. } => {
            infer_expr(object, env, semantic)?;
            Ok(info(Type::Unknown))
        }
        Expr::Call { callee, args } => {
            infer_expr(callee, env, semantic)?;
            for arg in args {
                infer_expr(arg, env, semantic)?;
            }
            Ok(info(Type::Unknown))
        }
        Expr::MethodCall { receiver, method, args } => {
            let receiver_info = infer_expr(receiver, env, semantic)?;
            for arg in args {
                infer_expr(arg, env, semantic)?;
            }
            resolve_method(&receiver_info.ty, method, semantic)
        }
    }
}

fn resolve_method(receiver: &Type, method: &str, semantic: &SemanticProgram) -> Result<ExprInfo, CompileError> {
    let core_matches = core::core_impls()
        .into_iter()
        .filter(|imp| imp.method == method && imp.target.matches(receiver))
        .collect::<Vec<_>>();
    let user_matches = semantic
        .impls
        .iter()
        .filter(|imp| {
            imp.target_type == *receiver
                && semantic
                    .traits
                    .get(&imp.source.trait_name)
                    .is_some_and(|trait_decl| trait_decl.methods.iter().any(|m| m.name == method))
        })
        .collect::<Vec<_>>();

    let count = core_matches.len() + user_matches.len();
    if count > 1 {
        return Err(CompileError::new(
            format!("ambiguous trait method `{method}` for type {receiver:?}"),
            Span::new(0, 0),
        ));
    }
    if let Some(imp) = core_matches.first() {
        return Ok(ExprInfo {
            ty: return_type_for_trait(imp.trait_name, method, semantic),
            lowering: Some(imp.lowering.clone()),
        });
    }
    if let Some(imp) = user_matches.first() {
        return Ok(ExprInfo {
            ty: return_type_for_trait(&imp.source.trait_name, method, semantic),
            lowering: Some(Lowering::Symbol {
                package: semantic.package_name.clone(),
                trait_name: imp.source.trait_name.clone(),
                method: method.to_string(),
            }),
        });
    }

    // Unknown/inherent JavaScript methods are preserved for interoperability.
    Ok(info(Type::Unknown))
}

fn return_type_for_trait(trait_name: &str, method: &str, semantic: &SemanticProgram) -> Type {
    semantic
        .traits
        .get(trait_name)
        .and_then(|t| t.methods.iter().find(|m| m.name == method))
        .and_then(|m| m.return_type.as_ref())
        .map(|ty| Type::from_ref(ty, &semantic.local_classes))
        .unwrap_or(Type::Unknown)
}

fn info(ty: Type) -> ExprInfo {
    ExprInfo { ty, lowering: None }
}
