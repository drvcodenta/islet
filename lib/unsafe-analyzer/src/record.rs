#[derive(Debug, Hash, Eq, PartialEq, Clone, Ord, PartialOrd)]
pub enum UnsafeKind {
    Function,
    Block,
    Trait,
    Impl,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct UnsafeItem {
    pub kind: UnsafeKind,
    pub name: String,
}

impl UnsafeItem {
    pub fn new(kind: UnsafeKind, name: String) -> Self {
        Self { kind, name }
    }
}
