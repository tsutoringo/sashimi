#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Function(Function),
    Trait(TraitDecl),
    Impl(ImplDecl),
    Class(ClassDecl),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub public: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeRef>,
    pub receiver: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDecl {
    pub public: bool,
    pub name: String,
    pub methods: Vec<TraitMethod>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplDecl {
    pub generics: Vec<String>,
    pub trait_name: String,
    pub target: TypeRef,
    pub methods: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDecl {
    pub public: bool,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Number(String),
    String(String),
    Bool(bool),
    Array(Vec<Expr>),
    Ident(String),
    New {
        class_name: String,
        args: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        property: String,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef {
    pub name: String,
    pub args: Vec<TypeRef>,
}

impl TypeRef {
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
        }
    }

    pub fn to_typescript(&self) -> String {
        let name = if self.name == "Iterator" {
            "SashimiIterator"
        } else {
            self.name.as_str()
        };

        if self.args.is_empty() {
            name.to_string()
        } else {
            format!(
                "{}<{}>",
                name,
                self.args
                    .iter()
                    .map(TypeRef::to_typescript)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.name == name || self.args.iter().any(|arg| arg.contains(name))
    }
}
