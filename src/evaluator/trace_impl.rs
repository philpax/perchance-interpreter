use crate::span::Span;
use crate::trace::{OperationType, TraceNode};
use rand::Rng;

use super::Evaluator;

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    /// Get the root trace node (call after evaluation completes)
    pub fn take_trace(&self) -> Option<TraceNode> {
        if !self.trace_enabled {
            return None;
        }
        // Return the root trace node if it exists
        self.trace_stack.first().cloned()
    }

    /// Start a new trace operation
    pub(super) fn trace_start(
        &mut self,
        operation: String,
        op_type: OperationType,
        span: Option<Span>,
    ) {
        if !self.trace_enabled {
            return;
        }
        let mut node = TraceNode::new(operation, String::new()).with_type(op_type);
        if let Some(s) = span {
            node = node.with_span(s);
        }
        // Propagate current source template and generator name to all child nodes
        if let Some(ref template) = self.current_source_template {
            node = node.with_source_template(template.clone());
        }
        if let Some(ref name) = self.current_generator_name {
            node = node.with_generator_name(name.clone());
        }
        self.trace_stack.push(node);
    }

    /// Complete the current trace operation with a result
    pub(super) fn trace_end(&mut self, result: String) {
        if !self.trace_enabled {
            return;
        }
        if let Some(mut node) = self.trace_stack.pop() {
            node.result = result;
            // If there's a parent, add this as a child; otherwise it's the root
            if let Some(parent) = self.trace_stack.last_mut() {
                parent.add_child(node);
            } else {
                // This is the root node, keep it
                self.trace_stack.push(node);
            }
        }
    }
}
