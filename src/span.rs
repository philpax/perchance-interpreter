/// Span tracking for source code positions
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// Represents a span in the source code (byte offsets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// Create a span that covers from the start of one span to the end of another
    pub fn merge(start: Span, end: Span) -> Self {
        Span {
            start: start.start,
            end: end.end,
        }
    }

    /// Convert to a Range for use with ariadne
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// Get the length of the span
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Create a dummy span (used for synthetic AST nodes)
    pub fn dummy() -> Self {
        Span { start: 0, end: 0 }
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Span {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

/// A value with an associated span
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Spanned { value, span }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }
}
