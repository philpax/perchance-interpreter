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

    /// Available options for list selections (text of each item)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_items: Option<Vec<String>>,

    /// Index of selected item (if this was a selection)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_index: Option<usize>,

    /// Template string showing where result was interpolated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolation_context: Option<String>,

    /// Source template for this generator (for root nodes or imports)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_template: Option<String>,

    /// Generator name (for imports or root)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator_name: Option<String>,

    /// Content of inline list for expandable display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_list_content: Option<String>,
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
            available_items: None,
            selected_index: None,
            interpolation_context: None,
            source_template: None,
            generator_name: None,
            inline_list_content: None,
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

    /// Set available items for a selection
    pub fn with_available_items(mut self, items: Vec<String>) -> Self {
        self.available_items = Some(items);
        self
    }

    /// Set the selected index
    pub fn with_selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Set interpolation context
    pub fn with_interpolation_context(mut self, context: String) -> Self {
        self.interpolation_context = Some(context);
        self
    }

    /// Set source template
    pub fn with_source_template(mut self, template: String) -> Self {
        self.source_template = Some(template);
        self
    }

    /// Set generator name
    pub fn with_generator_name(mut self, name: String) -> Self {
        self.generator_name = Some(name);
        self
    }

    /// Set inline list content
    pub fn with_inline_list_content(mut self, content: String) -> Self {
        self.inline_list_content = Some(content);
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
