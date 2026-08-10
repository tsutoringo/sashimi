use crate::{
    ast::{Param, TraitDecl, TraitMethod, TypeRef},
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lowering {
    Property(&'static str),
    IteratorFromIterable,
    IteratorMethod(&'static str),
    Symbol {
        package: String,
        trait_name: String,
        method: String,
    },
}

#[derive(Debug, Clone)]
pub struct CoreImpl {
    pub trait_name: &'static str,
    pub method: &'static str,
    pub target: CoreTarget,
    pub arity: usize,
    pub lowering: Lowering,
}

#[derive(Debug, Clone, Copy)]
pub enum CoreTarget {
    Array,
    String,
    Map,
    Set,
    Iterator,
}

pub fn prelude_traits() -> Vec<TraitDecl> {
    vec![
        trait_decl(
            "Len",
            vec![trait_method("len", &[], TypeRef::simple("number"))],
        ),
        trait_decl(
            "IntoIterator",
            vec![trait_method(
                "iter",
                &[],
                TypeRef {
                    name: "Iterator".to_string(),
                    args: vec![TypeRef::simple("unknown")],
                },
            )],
        ),
        trait_decl(
            "Iterator",
            vec![
                trait_method("next", &[], TypeRef::simple("unknown")),
                trait_method(
                    "map",
                    &["mapper"],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method(
                    "filter",
                    &["predicate"],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method("take", &["count"], iterator_ref(TypeRef::simple("unknown"))),
                trait_method("skip", &["count"], iterator_ref(TypeRef::simple("unknown"))),
                trait_method(
                    "enumerate",
                    &[],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method(
                    "chain",
                    &["other"],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method(
                    "zip",
                    &["other"],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method(
                    "inspect",
                    &["callback"],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method(
                    "flat_map",
                    &["mapper"],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method(
                    "flatten",
                    &[],
                    iterator_ref(TypeRef::simple("unknown")),
                ),
                trait_method(
                    "collect",
                    &[],
                    TypeRef {
                        name: "Array".to_string(),
                        args: vec![TypeRef::simple("unknown")],
                    },
                ),
                trait_method("count", &[], TypeRef::simple("number")),
                trait_method("nth", &["index"], TypeRef::simple("unknown")),
                trait_method("last", &[], TypeRef::simple("unknown")),
                trait_method("find", &["predicate"], TypeRef::simple("unknown")),
                trait_method("position", &["predicate"], TypeRef::simple("unknown")),
                trait_method("any", &["predicate"], TypeRef::simple("boolean")),
                trait_method("all", &["predicate"], TypeRef::simple("boolean")),
                trait_method("fold", &["initial", "folder"], TypeRef::simple("unknown")),
                trait_method("reduce", &["reducer"], TypeRef::simple("unknown")),
                trait_method("sum", &[], TypeRef::simple("number")),
                trait_method("product", &[], TypeRef::simple("number")),
                trait_method("min", &[], TypeRef::simple("unknown")),
                trait_method("max", &[], TypeRef::simple("unknown")),
                trait_method("for_each", &["callback"], TypeRef::simple("unknown")),
            ],
        ),
    ]
}

pub fn core_impls() -> Vec<CoreImpl> {
    let mut impls = vec![
        core_impl("Len", "len", CoreTarget::Array, 0, Lowering::Property("length")),
        core_impl("Len", "len", CoreTarget::String, 0, Lowering::Property("length")),
        core_impl("Len", "len", CoreTarget::Map, 0, Lowering::Property("size")),
        core_impl("Len", "len", CoreTarget::Set, 0, Lowering::Property("size")),
        core_impl(
            "IntoIterator",
            "iter",
            CoreTarget::Array,
            0,
            Lowering::IteratorFromIterable,
        ),
        core_impl(
            "IntoIterator",
            "iter",
            CoreTarget::Map,
            0,
            Lowering::IteratorFromIterable,
        ),
    ];

    for (method, arity) in [
        ("next", 0),
        ("map", 1),
        ("filter", 1),
        ("take", 1),
        ("skip", 1),
        ("enumerate", 0),
        ("chain", 1),
        ("zip", 1),
        ("inspect", 1),
        ("flat_map", 1),
        ("flatten", 0),
        ("collect", 0),
        ("count", 0),
        ("nth", 1),
        ("last", 0),
        ("find", 1),
        ("position", 1),
        ("any", 1),
        ("all", 1),
        ("fold", 2),
        ("reduce", 1),
        ("sum", 0),
        ("product", 0),
        ("min", 0),
        ("max", 0),
        ("for_each", 1),
    ] {
        impls.push(core_impl(
            "Iterator",
            method,
            CoreTarget::Iterator,
            arity,
            Lowering::IteratorMethod(method),
        ));
    }

    impls
}

pub fn return_type(imp: &CoreImpl, receiver: &Type) -> Type {
    match (imp.trait_name, imp.method) {
        ("Len", "len") => Type::Number,
        ("IntoIterator", "iter") => match receiver {
            Type::Array(element) => Type::Iterator(element.clone()),
            Type::Map(key, value) => Type::Iterator(Box::new(Type::Tuple(vec![
                key.as_ref().clone(),
                value.as_ref().clone(),
            ]))),
            _ => Type::Iterator(Box::new(Type::Unknown)),
        },
        ("Iterator", "filter" | "take" | "skip" | "inspect" | "chain") => receiver.clone(),
        ("Iterator", "map" | "flat_map" | "flatten") => Type::Iterator(Box::new(Type::Unknown)),
        ("Iterator", "enumerate") => Type::Iterator(Box::new(Type::Tuple(vec![
            Type::Number,
            iterator_element(receiver),
        ]))),
        ("Iterator", "zip") => Type::Iterator(Box::new(Type::Tuple(vec![
            iterator_element(receiver),
            Type::Unknown,
        ]))),
        ("Iterator", "collect") => Type::Array(Box::new(iterator_element(receiver))),
        ("Iterator", "count" | "sum" | "product") => Type::Number,
        ("Iterator", "any" | "all") => Type::Boolean,
        _ => Type::Unknown,
    }
}

impl CoreTarget {
    pub fn matches(self, ty: &Type) -> bool {
        match (self, ty) {
            (CoreTarget::Array, Type::Array(_)) => true,
            (CoreTarget::String, Type::String) => true,
            (CoreTarget::Map, Type::Map(_, _)) => true,
            (CoreTarget::Map, Type::External(name)) => name == "Map",
            (CoreTarget::Set, Type::Set(_)) => true,
            (CoreTarget::Set, Type::External(name)) => name == "Set",
            (CoreTarget::Iterator, Type::Iterator(_)) => true,
            _ => false,
        }
    }
}

pub fn is_foreign_builtin(name: &str) -> bool {
    matches!(
        name,
        "Array"
            | "String"
            | "Map"
            | "Set"
            | "Promise"
            | "Date"
            | "RegExp"
            | "Uint8Array"
            | "Int8Array"
            | "Uint16Array"
            | "Int16Array"
            | "Uint32Array"
            | "Int32Array"
            | "Float32Array"
            | "Float64Array"
    )
}

fn core_impl(
    trait_name: &'static str,
    method: &'static str,
    target: CoreTarget,
    arity: usize,
    lowering: Lowering,
) -> CoreImpl {
    CoreImpl {
        trait_name,
        method,
        target,
        arity,
        lowering,
    }
}

fn trait_decl(name: &str, methods: Vec<TraitMethod>) -> TraitDecl {
    TraitDecl {
        public: true,
        name: name.to_string(),
        methods,
    }
}

fn trait_method(name: &str, params: &[&str], return_type: TypeRef) -> TraitMethod {
    let mut all_params = vec![Param {
        name: "self".to_string(),
        ty: None,
        receiver: true,
    }];
    all_params.extend(params.iter().map(|name| Param {
        name: (*name).to_string(),
        ty: None,
        receiver: false,
    }));

    TraitMethod {
        name: name.to_string(),
        params: all_params,
        return_type: Some(return_type),
    }
}

fn iterator_ref(item: TypeRef) -> TypeRef {
    TypeRef {
        name: "Iterator".to_string(),
        args: vec![item],
    }
}

fn iterator_element(receiver: &Type) -> Type {
    match receiver {
        Type::Iterator(element) => element.as_ref().clone(),
        _ => Type::Unknown,
    }
}
