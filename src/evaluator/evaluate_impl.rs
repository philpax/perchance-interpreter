use crate::span::Span;
use crate::trace::OperationType;
use rand::Rng;

use super::{EvalError, Evaluator};

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    pub async fn evaluate(&mut self) -> Result<String, EvalError> {
        // Start root trace
        self.trace_start("Evaluate program".to_string(), OperationType::Root, None);

        // Priority order: $output, output, then last list
        // Check for $output list first (top-level $output = ...)
        let result = if let Some(output_list) = self.program.get_list("$output") {
            self.evaluate_list(output_list, None).await
        } else {
            // Check for output list
            match self.program.get_list("output") {
                Some(output_list) => self.evaluate_list(output_list, None).await,
                None => {
                    // Default to the last list if no "output" list is defined
                    if let Some(last_list_name) = self.program.list_order.last() {
                        if let Some(last_list) = self.program.get_list(last_list_name) {
                            self.evaluate_list(last_list, None).await
                        } else {
                            Err(EvalError::UndefinedList {
                                name: "output".to_string(),
                                span: Span::dummy(),
                            })
                        }
                    } else {
                        Err(EvalError::UndefinedList {
                            name: "output".to_string(),
                            span: Span::dummy(),
                        })
                    }
                }
            }
        };

        // End root trace
        if let Ok(ref output) = result {
            self.trace_end(output.clone());
        }

        result
    }
}
