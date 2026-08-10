use std::collections::{HashMap, HashSet};

use crate::{
    ast::{Expr, Function, Item, Stmt, TypeRef},
    core::Lowering,
    diagnostic::{CompileError, Span},
    semantic::{SemanticProgram, infer_expr},
    types::Type,
};

pub struct Generated {
    pub js: String,
    pub dts: String,
}

pub fn generate(semantic: &SemanticProgram) -> Result<Generated, CompileError> {
    let mut js = String::new();
    let mut dts = String::new();
    let mut needs_iterator_runtime = false;

    emit_symbol_declarations(semantic, &mut js);

    // Classes must exist before symbol-backed impls mutate their prototypes.
    for item in &semantic.program.items {
        if let Item::Class(class) = item {
            if class.public {
                js.push_str("export ");
            }
            js.push_str(&format!("class {} {{}}\n", class.name));
            if class.public {
                dts.push_str(&format!("export declare class {} {{}}\n", class.name));
            }
        }
    }

    for item in &semantic.program.items {
        if let Item::Impl(imp) = item {
            emit_impl(
                imp,
                semantic,
                &mut js,
                &mut needs_iterator_runtime,
            )?;
        }
    }

    for item in &semantic.program.items {
        if let Item::Function(function) = item {
            emit_function(
                function,
                semantic,
                &mut js,
                &mut needs_iterator_runtime,
            )?;
            if function.public {
                emit_dts_function(function, &mut dts);
            }
        }
    }

    if needs_iterator_runtime {
        js = format!("{ITERATOR_RUNTIME}\n{js}");
    }
    if semantic.program.items.iter().any(|item| {
        matches!(item, Item::Function(function) if function.public && function_uses_iterator_type(function))
    }) {
        dts = format!("{ITERATOR_DTS}\n{dts}");
    }

    Ok(Generated { js, dts })
}

fn emit_symbol_declarations(semantic: &SemanticProgram, js: &mut String) {
    let mut emitted = HashSet::new();
    for imp in &semantic.impls {
        for method in &imp.source.methods {
            let identity = (imp.source.trait_name.clone(), method.name.clone());
            if !emitted.insert(identity) {
                continue;
            }
            let symbol = symbol_ident(&imp.source.trait_name, &method.name);
            let key = format!(
                "sashimi:{}::{}.{}",
                semantic.package_name, imp.source.trait_name, method.name
            );
            js.push_str(&format!("const {symbol} = Symbol.for({key:?});\n"));
        }
    }
    if !emitted.is_empty() {
        js.push('\n');
    }
}

fn emit_impl(
    imp: &crate::ast::ImplDecl,
    semantic: &SemanticProgram,
    js: &mut String,
    needs_iterator_runtime: &mut bool,
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
        emit_body(
            &method.body,
            semantic,
            &mut env,
            js,
            1,
            needs_iterator_runtime,
        )?;
        js.push_str("};\n\n");
    }
    Ok(())
}

fn emit_function(
    function: &Function,
    semantic: &SemanticProgram,
    js: &mut String,
    needs_iterator_runtime: &mut bool,
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
    emit_body(
        &function.body,
        semantic,
        &mut env,
        js,
        1,
        needs_iterator_runtime,
    )?;
    js.push_str("}\n\n");
    Ok(())
}

fn emit_body(
    statements: &[Stmt],
    semantic: &SemanticProgram,
    env: &mut HashMap<String, Type>,
    js: &mut String,
    indent: usize,
    needs_iterator_runtime: &mut bool,
) -> Result<(), CompileError> {
    for statement in statements {
        js.push_str(&"    ".repeat(indent));
        match statement {
            Stmt::Let { name, value } => {
                let info = infer_expr(value, env, semantic)?;
                js.push_str(&format!(
                    "const {name} = {}",
                    emit_expr(value, semantic, env, needs_iterator_runtime)?
                ));
                env.insert(name.clone(), info.ty);
            }
            Stmt::Expr(expr) => js.push_str(&emit_expr(
                expr,
                semantic,
                env,
                needs_iterator_runtime,
            )?),
            Stmt::Return(Some(expr)) => {
                js.push_str(&format!(
                    "return {}",
                    emit_expr(expr, semantic, env, needs_iterator_runtime)?
                ));
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
    needs_iterator_runtime: &mut bool,
) -> Result<String, CompileError> {
    match expr {
        Expr::Number(value) => Ok(value.clone()),
        Expr::String(value) => Ok(format!("{value:?}")),
        Expr::Bool(value) => Ok(value.to_string()),
        Expr::Array(items) => Ok(format!(
            "[{}]",
            items
                .iter()
                .map(|item| emit_expr(item, semantic, env, needs_iterator_runtime))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        Expr::Ident(name) if name == "self" => Ok("this".to_string()),
        Expr::Ident(name) => Ok(name.clone()),
        Expr::New { class_name, args } => Ok(format!(
            "new {class_name}({})",
            emit_args(args, semantic, env, needs_iterator_runtime)?
        )),
        Expr::Member { object, property } => Ok(format!(
            "{}.{}",
            emit_expr(object, semantic, env, needs_iterator_runtime)?,
            property
        )),
        Expr::Call { callee, args } => Ok(format!(
            "{}({})",
            emit_expr(callee, semantic, env, needs_iterator_runtime)?,
            emit_args(args, semantic, env, needs_iterator_runtime)?
        )),
        Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            let receiver_js = emit_expr(receiver, semantic, env, needs_iterator_runtime)?;
            let info = infer_expr(expr, env, semantic)?;
            match info.lowering {
                Some(Lowering::Property(property)) if args.is_empty() => {
                    Ok(format!("{receiver_js}.{property}"))
                }
                Some(Lowering::Property(_)) => Err(CompileError::new(
                    "property intrinsic does not accept arguments",
                    Span::new(0, 0),
                )),
                Some(Lowering::IteratorFromIterable) => {
                    *needs_iterator_runtime = true;
                    if !args.is_empty() {
                        return Err(CompileError::new(
                            "iter() does not accept arguments",
                            Span::new(0, 0),
                        ));
                    }
                    Ok(format!("__sashimi_iter({receiver_js})"))
                }
                Some(Lowering::IteratorMethod(runtime_method)) => {
                    *needs_iterator_runtime = true;
                    Ok(format!(
                        "{receiver_js}.{runtime_method}({})",
                        emit_args(args, semantic, env, needs_iterator_runtime)?
                    ))
                }
                Some(Lowering::Symbol {
                    trait_name,
                    method,
                    ..
                }) => Ok(format!(
                    "{receiver_js}[{}]({})",
                    symbol_ident(&trait_name, &method),
                    emit_args(args, semantic, env, needs_iterator_runtime)?
                )),
                None => Ok(format!(
                    "{receiver_js}.{method}({})",
                    emit_args(args, semantic, env, needs_iterator_runtime)?
                )),
            }
        }
    }
}

fn emit_args(
    args: &[Expr],
    semantic: &SemanticProgram,
    env: &HashMap<String, Type>,
    needs_iterator_runtime: &mut bool,
) -> Result<String, CompileError> {
    Ok(args
        .iter()
        .map(|arg| emit_expr(arg, semantic, env, needs_iterator_runtime))
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
                    .map_or("unknown".to_string(), TypeRef::to_typescript)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret = function
        .return_type
        .as_ref()
        .map_or("void".to_string(), TypeRef::to_typescript);
    dts.push_str(&format!(
        "export declare function {}({params}): {ret};\n",
        function.name
    ));
}

fn function_uses_iterator_type(function: &Function) -> bool {
    function
        .params
        .iter()
        .filter_map(|param| param.ty.as_ref())
        .any(|ty| ty.contains("Iterator"))
        || function
            .return_type
            .as_ref()
            .is_some_and(|ty| ty.contains("Iterator"))
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

const ITERATOR_DTS: &str = r#"export interface SashimiIterator<T> extends IterableIterator<T> {
    map<U>(mapper: (value: T) => U): SashimiIterator<U>;
    filter(predicate: (value: T) => boolean): SashimiIterator<T>;
    take(count: number): SashimiIterator<T>;
    skip(count: number): SashimiIterator<T>;
    enumerate(): SashimiIterator<[number, T]>;
    chain(other: Iterable<T>): SashimiIterator<T>;
    zip<U>(other: Iterable<U>): SashimiIterator<[T, U]>;
    inspect(callback: (value: T) => void): SashimiIterator<T>;
    flat_map<U>(mapper: (value: T) => Iterable<U>): SashimiIterator<U>;
    flatten<U>(this: SashimiIterator<Iterable<U>>): SashimiIterator<U>;
    collect(): T[];
    count(): number;
    nth(index: number): T | undefined;
    last(): T | undefined;
    find(predicate: (value: T) => boolean): T | undefined;
    position(predicate: (value: T) => boolean): number | undefined;
    any(predicate: (value: T) => boolean): boolean;
    all(predicate: (value: T) => boolean): boolean;
    fold<U>(initial: U, folder: (accumulator: U, value: T) => U): U;
    reduce(reducer: (accumulator: T, value: T) => T): T | undefined;
    sum(this: SashimiIterator<number>): number;
    product(this: SashimiIterator<number>): number;
    min(): T | undefined;
    max(): T | undefined;
    for_each(callback: (value: T) => void): void;
}
"#;

const ITERATOR_RUNTIME: &str = r#"class __SashimiIterator {
    constructor(iterator) {
        this.__iterator = iterator;
    }

    [Symbol.iterator]() {
        return this;
    }

    next() {
        return this.__iterator.next();
    }

    map(mapper) {
        const source = this;
        return __sashimi_iter((function* () {
            for (const value of source) {
                yield mapper(value);
            }
        })());
    }

    filter(predicate) {
        const source = this;
        return __sashimi_iter((function* () {
            for (const value of source) {
                if (predicate(value)) {
                    yield value;
                }
            }
        })());
    }

    take(count) {
        const source = this;
        return __sashimi_iter((function* () {
            let remaining = Math.max(0, count);
            while (remaining > 0) {
                const next = source.next();
                if (next.done) {
                    return;
                }
                remaining -= 1;
                yield next.value;
            }
        })());
    }

    skip(count) {
        const source = this;
        return __sashimi_iter((function* () {
            let remaining = Math.max(0, count);
            while (remaining > 0) {
                const next = source.next();
                if (next.done) {
                    return;
                }
                remaining -= 1;
            }
            yield* source;
        })());
    }

    enumerate() {
        const source = this;
        return __sashimi_iter((function* () {
            let index = 0;
            for (const value of source) {
                yield [index, value];
                index += 1;
            }
        })());
    }

    chain(other) {
        const source = this;
        return __sashimi_iter((function* () {
            yield* source;
            yield* __sashimi_iter(other);
        })());
    }

    zip(other) {
        const source = this;
        const rhs = __sashimi_iter(other);
        return __sashimi_iter((function* () {
            while (true) {
                const left = source.next();
                const right = rhs.next();
                if (left.done || right.done) {
                    return;
                }
                yield [left.value, right.value];
            }
        })());
    }

    inspect(callback) {
        const source = this;
        return __sashimi_iter((function* () {
            for (const value of source) {
                callback(value);
                yield value;
            }
        })());
    }

    flat_map(mapper) {
        const source = this;
        return __sashimi_iter((function* () {
            for (const value of source) {
                yield* __sashimi_iter(mapper(value));
            }
        })());
    }

    flatten() {
        const source = this;
        return __sashimi_iter((function* () {
            for (const value of source) {
                yield* __sashimi_iter(value);
            }
        })());
    }

    collect() {
        return Array.from(this);
    }

    count() {
        let count = 0;
        for (const _value of this) {
            count += 1;
        }
        return count;
    }

    nth(index) {
        if (index < 0) {
            return undefined;
        }
        let current = 0;
        for (const value of this) {
            if (current === index) {
                return value;
            }
            current += 1;
        }
        return undefined;
    }

    last() {
        let found = false;
        let lastValue;
        for (const value of this) {
            found = true;
            lastValue = value;
        }
        return found ? lastValue : undefined;
    }

    find(predicate) {
        for (const value of this) {
            if (predicate(value)) {
                return value;
            }
        }
        return undefined;
    }

    position(predicate) {
        let index = 0;
        for (const value of this) {
            if (predicate(value)) {
                return index;
            }
            index += 1;
        }
        return undefined;
    }

    any(predicate) {
        for (const value of this) {
            if (predicate(value)) {
                return true;
            }
        }
        return false;
    }

    all(predicate) {
        for (const value of this) {
            if (!predicate(value)) {
                return false;
            }
        }
        return true;
    }

    fold(initial, folder) {
        let accumulator = initial;
        for (const value of this) {
            accumulator = folder(accumulator, value);
        }
        return accumulator;
    }

    reduce(reducer) {
        const first = this.next();
        if (first.done) {
            return undefined;
        }
        let accumulator = first.value;
        for (const value of this) {
            accumulator = reducer(accumulator, value);
        }
        return accumulator;
    }

    sum() {
        return this.fold(0, (accumulator, value) => accumulator + value);
    }

    product() {
        return this.fold(1, (accumulator, value) => accumulator * value);
    }

    min() {
        return this.reduce((best, value) => value < best ? value : best);
    }

    max() {
        return this.reduce((best, value) => value > best ? value : best);
    }

    for_each(callback) {
        for (const value of this) {
            callback(value);
        }
    }
}

function __sashimi_iter(iterable) {
    if (iterable instanceof __SashimiIterator) {
        return iterable;
    }
    if (iterable == null) {
        throw new TypeError("value is not iterable");
    }
    if (typeof iterable.next === "function") {
        return new __SashimiIterator(iterable);
    }
    const iteratorFactory = iterable[Symbol.iterator];
    if (typeof iteratorFactory !== "function") {
        throw new TypeError("value is not iterable");
    }
    return new __SashimiIterator(iteratorFactory.call(iterable));
}
"#;
