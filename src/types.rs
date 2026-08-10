use crate::ast::TypeRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unknown,
    Number,
    String,
    Boolean,
    Array(Box<Type>),
    LocalClass(String),
    External(String),
}

impl Type {
    pub fn from_ref(reference: &TypeRef, local_classes: &std::collections::HashSet<String>) -> Self {
        match reference.name.as_str() {
            "number" => Type::Number,
            "string" => Type::String,
            "boolean" => Type::Boolean,
            "Array" => Type::Array(Box::new(
                reference
                    .args
                    .first()
                    .map_or(Type::Unknown, |x| Type::from_ref(x, local_classes)),
            )),
            name if local_classes.contains(name) => Type::LocalClass(name.to_string()),
            name => Type::External(name.to_string()),
        }
    }
}
