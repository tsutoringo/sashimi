use crate::ast::TypeRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unknown,
    Number,
    String,
    Boolean,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Set(Box<Type>),
    Iterator(Box<Type>),
    Tuple(Vec<Type>),
    LocalClass(String),
    External(String),
}

impl Type {
    pub fn from_ref(reference: &TypeRef, local_classes: &std::collections::HashSet<String>) -> Self {
        match reference.name.as_str() {
            "unknown" => Type::Unknown,
            "number" => Type::Number,
            "string" => Type::String,
            "boolean" => Type::Boolean,
            "Array" => Type::Array(Box::new(type_arg(reference, 0, local_classes))),
            "Map" => Type::Map(
                Box::new(type_arg(reference, 0, local_classes)),
                Box::new(type_arg(reference, 1, local_classes)),
            ),
            "Set" => Type::Set(Box::new(type_arg(reference, 0, local_classes))),
            "Iterator" => Type::Iterator(Box::new(type_arg(reference, 0, local_classes))),
            name if local_classes.contains(name) => Type::LocalClass(name.to_string()),
            name => Type::External(name.to_string()),
        }
    }
}

fn type_arg(
    reference: &TypeRef,
    index: usize,
    local_classes: &std::collections::HashSet<String>,
) -> Type {
    reference
        .args
        .get(index)
        .map_or(Type::Unknown, |arg| Type::from_ref(arg, local_classes))
}
