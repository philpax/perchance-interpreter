use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    UndefinedList {
        name: String,
        span: Span,
    },
    UndefinedVariable {
        name: String,
        span: Span,
    },
    UndefinedProperty {
        list: String,
        prop: String,
        span: Span,
    },
    InvalidMethodCall {
        message: String,
        span: Span,
    },
    EmptyList {
        name: String,
        span: Span,
    },
    TypeError {
        message: String,
        span: Span,
    },
    ImportError {
        message: String,
        span: Span,
    },
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UndefinedList { name, span } => {
                write!(f, "Undefined list: {} at position {}", name, span.start)
            }
            EvalError::UndefinedVariable { name, span } => {
                write!(f, "Undefined variable: {} at position {}", name, span.start)
            }
            EvalError::UndefinedProperty { list, prop, span } => {
                write!(
                    f,
                    "Undefined property '{}' on list '{}' at position {}",
                    prop, list, span.start
                )
            }
            EvalError::InvalidMethodCall { message, span } => {
                write!(
                    f,
                    "Invalid method call: {} at position {}",
                    message, span.start
                )
            }
            EvalError::EmptyList { name, span } => {
                write!(
                    f,
                    "Cannot select from empty list: {} at position {}",
                    name, span.start
                )
            }
            EvalError::TypeError { message, span } => {
                write!(f, "Type error: {} at position {}", message, span.start)
            }
            EvalError::ImportError { message, span } => {
                write!(f, "Import error: {} at position {}", message, span.start)
            }
        }
    }
}

impl std::error::Error for EvalError {}

impl EvalError {
    /// Get the span associated with this error
    pub fn span(&self) -> Span {
        match self {
            EvalError::UndefinedList { span, .. } => *span,
            EvalError::UndefinedVariable { span, .. } => *span,
            EvalError::UndefinedProperty { span, .. } => *span,
            EvalError::InvalidMethodCall { span, .. } => *span,
            EvalError::EmptyList { span, .. } => *span,
            EvalError::TypeError { span, .. } => *span,
            EvalError::ImportError { span, .. } => *span,
        }
    }
}
