/// Trace functionality for debugging and visualizing evaluation
use crate::span::Span;
use serde::{Deserialize, Serialize};

/// Represents a single step in the evaluation trace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceNode {
    /// Human-readable description of what was evaluated
    pub operation: String,

    /// The result produced by this operation
    pub result: String,

    /// Position in the original template (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,

    /// RNG seed used for this operation (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rng_seed: Option<u64>,

    /// Child traces (nested operations)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TraceNode>,

    /// Additional metadata about the operation type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_type: Option<OperationType>,
}

/// Types of operations that can be traced
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    /// List selection: [listname]
    ListSelect,

    /// Variable assignment: x = value
    Assignment,

    /// Property access: list.property
    PropertyAccess,

    /// Method call: value.method()
    MethodCall,

    /// Range selection: {1-10}
    Range,

    /// Choice selection: {a|b|c}
    Choice,

    /// Import: {import:generator}
    Import,

    /// Conditional: [condition ? true : false]
    Conditional,

    /// Repeat loop
    Repeat,

    /// String concatenation
    Concatenation,

    /// Root evaluation
    Root,
}

impl TraceNode {
    /// Create a new trace node
    pub fn new(operation: String, result: String) -> Self {
        TraceNode {
            operation,
            result,
            span: None,
            rng_seed: None,
            children: Vec::new(),
            operation_type: None,
        }
    }

    /// Create a trace node with a specific operation type
    pub fn with_type(mut self, op_type: OperationType) -> Self {
        self.operation_type = Some(op_type);
        self
    }

    /// Add span information
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Add RNG seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng_seed = Some(seed);
        self
    }

    /// Add a child trace node
    pub fn add_child(&mut self, child: TraceNode) {
        self.children.push(child);
    }

    /// Add multiple children
    pub fn with_children(mut self, children: Vec<TraceNode>) -> Self {
        self.children = children;
        self
    }
}

/// Result type that includes trace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceResult {
    /// The final output
    pub output: String,

    /// The complete trace tree
    pub trace: TraceNode,
}

impl TraceResult {
    pub fn new(output: String, trace: TraceNode) -> Self {
        TraceResult { output, trace }
    }
}
