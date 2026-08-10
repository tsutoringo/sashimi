use crate::types::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lowering {
    Property(&'static str),
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
    pub lowering: Lowering,
}

#[derive(Debug, Clone, Copy)]
pub enum CoreTarget {
    Array,
    String,
    Map,
    Set,
}

pub fn prelude_traits() -> &'static [&'static str] {
    &["Len"]
}

pub fn core_impls() -> Vec<CoreImpl> {
    vec![
        CoreImpl {
            trait_name: "Len",
            method: "len",
            target: CoreTarget::Array,
            lowering: Lowering::Property("length"),
        },
        CoreImpl {
            trait_name: "Len",
            method: "len",
            target: CoreTarget::String,
            lowering: Lowering::Property("length"),
        },
        CoreImpl {
            trait_name: "Len",
            method: "len",
            target: CoreTarget::Map,
            lowering: Lowering::Property("size"),
        },
        CoreImpl {
            trait_name: "Len",
            method: "len",
            target: CoreTarget::Set,
            lowering: Lowering::Property("size"),
        },
    ]
}

impl CoreTarget {
    pub fn matches(self, ty: &Type) -> bool {
        match (self, ty) {
            (CoreTarget::Array, Type::Array(_)) => true,
            (CoreTarget::String, Type::String) => true,
            (CoreTarget::Map, Type::External(name)) => name == "Map",
            (CoreTarget::Set, Type::External(name)) => name == "Set",
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
