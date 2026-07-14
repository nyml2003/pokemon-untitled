use crate::model::{MethodName, NodePath, ParameterName};
use crate::schema::Value;

/// A half-open byte range in the original source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub const fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

/// Parsed shell text. It is syntax only and has not been resolved or validated
/// against a registry or method schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub calls: Vec<Call>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub path: Spanned<NodePath>,
    pub method: Spanned<MethodName>,
    pub arguments: Vec<Argument>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Argument {
    Positional(Spanned<Value>),
    Named {
        name: Spanned<ParameterName>,
        value: Spanned<Value>,
        span: Span,
    },
}

impl Argument {
    pub const fn span(&self) -> Span {
        match self {
            Self::Positional(value) => value.span,
            Self::Named { span, .. } => *span,
        }
    }
}
