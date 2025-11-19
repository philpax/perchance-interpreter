//! List evaluation implementation for the Evaluator
//!
//! This module contains methods for evaluating lists, selecting weighted items,
//! and converting lists to values in the Perchance interpreter.

use async_recursion::async_recursion;
use rand::Rng;

use crate::ast::{ContentPart, Expression};
use crate::compiler::{CompiledItem, CompiledList};
use crate::span::Span;
use crate::trace::OperationType;

use super::value::Value;
use super::{EvalError, Evaluator};

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    /// Evaluate a list by selecting a weighted item and evaluating its content
    ///
    /// This method handles:
    /// - Empty list validation
    /// - Weighted random selection from list items
    /// - Trace collection for debugging
    /// - $output property evaluation
    /// - Sublist selection and evaluation
    /// - Item content evaluation
    ///
    /// # Arguments
    ///
    /// * `list` - The compiled list to evaluate
    /// * `span` - Optional span information for error reporting
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The evaluated result
    /// * `Err(EvalError)` - An error if evaluation fails
    #[async_recursion]
    pub(super) async fn evaluate_list(
        &mut self,
        list: &CompiledList,
        span: Option<Span>,
    ) -> Result<String, EvalError> {
        // Start tracing this list evaluation
        self.trace_start(format!("[{}]", list.name), OperationType::ListSelect, span);

        if list.items.is_empty() && list.output.is_none() {
            return Err(EvalError::EmptyList {
                name: list.name.clone(),
                span: Span::dummy(),
            });
        }

        // Select an item based on weights (if there are items)
        let (item_opt, _selected_idx) = if !list.items.is_empty() {
            // Get previews of all items for trace
            let item_previews: Vec<String> = list
                .items
                .iter()
                .map(|item| self.get_item_preview(&item.content))
                .collect();

            let (selected_item, idx) = self
                .select_weighted_item(&list.items, list.total_weight)
                .await?;

            // Store trace info about the selection
            if self.trace_enabled {
                if let Some(node) = self.trace_stack.last_mut() {
                    node.available_items = Some(item_previews);
                    node.selected_index = Some(idx);
                }
            }

            (Some(selected_item.clone()), idx)
        } else {
            (None, 0)
        };

        // Check if list has $output property
        if let Some(output_content) = &list.output {
            // Set current_item for `this` keyword access
            let old_item = self.current_item.take();
            let old_dynamic_properties = std::mem::take(&mut self.dynamic_properties);

            if let Some(ref selected_item) = item_opt {
                self.current_item = Some(selected_item.clone());
            }

            let result = self.evaluate_content(output_content).await;

            // Restore previous context
            self.current_item = old_item;
            self.dynamic_properties = old_dynamic_properties;

            // End trace
            if let Ok(ref output) = result {
                self.trace_end(output.clone());
            }

            return result;
        }

        // No $output, use normal evaluation
        let item = item_opt.unwrap(); // Safe because we checked items.is_empty() above

        // If the item has sublists, first select a sublist, then select from it
        if !item.sublists.is_empty() {
            // Randomly select a sublist
            let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
            let idx = self.rng.gen_range(0..sublist_names.len());
            let sublist_name = &sublist_names[idx];
            let sublist = item.sublists.get(sublist_name).unwrap();
            let result = self.evaluate_list(sublist, None).await;

            // End trace
            if let Ok(ref output) = result {
                self.trace_end(output.clone());
            }

            return result;
        }

        // Evaluate the item's content
        let result = self.evaluate_content(&item.content).await;

        // End trace
        if let Ok(ref output) = result {
            self.trace_end(output.clone());
        }

        result
    }

    /// Select a weighted item from a list of items
    ///
    /// This method handles:
    /// - Dynamic weight expression evaluation
    /// - Boolean to number conversion (true -> 1.0, false -> 0.0)
    /// - Zero-weight handling (equal weights fallback)
    /// - Weighted random selection
    ///
    /// # Arguments
    ///
    /// * `items` - The list of items to select from
    /// * `_total_weight` - The pre-calculated total weight (unused due to dynamic weights)
    ///
    /// # Returns
    ///
    /// * `Ok((&CompiledItem, usize))` - The selected item and its index
    /// * `Err(EvalError)` - An error if the list is empty or evaluation fails
    #[async_recursion]
    pub(super) async fn select_weighted_item<'b>(
        &mut self,
        items: &'b [CompiledItem],
        _total_weight: f64,
    ) -> Result<(&'b CompiledItem, usize), EvalError> {
        if items.is_empty() {
            return Err(EvalError::EmptyList {
                name: "(anonymous)".to_string(),
                span: Span::dummy(),
            });
        }

        // Calculate actual weights for items with dynamic weights
        let mut actual_weights: Vec<f64> = Vec::new();
        let mut actual_total = 0.0;

        for item in items {
            let weight = if let Some(ref expr) = item.dynamic_weight {
                // Evaluate the dynamic weight expression
                let result = self.evaluate_expression(expr).await?;
                // Convert to number: "true" -> 1.0, "false" -> 0.0, or parse as number
                let weight = if result == "true" {
                    1.0
                } else if result == "false" || result.is_empty() {
                    0.0
                } else {
                    result.parse::<f64>().unwrap_or(0.0)
                };
                weight.max(0.0)
            } else {
                item.weight
            };
            actual_weights.push(weight);
            actual_total += weight;
        }

        if actual_total <= 0.0 {
            // If all weights are 0, treat all items as having equal weight (1.0)
            actual_weights = vec![1.0; items.len()];
            actual_total = items.len() as f64;
        }

        let random_value = self.rng.gen::<f64>() * actual_total;
        let mut cumulative = 0.0;

        for (i, weight) in actual_weights.iter().enumerate() {
            cumulative += weight;
            if random_value < cumulative {
                return Ok((&items[i], i));
            }
        }

        // Fallback to last item (in case of floating point errors)
        let last_idx = items.len() - 1;
        Ok((&items[last_idx], last_idx))
    }

    /// Evaluate a list to a Value type
    ///
    /// This method handles converting a list to a Value, with special handling
    /// for imported generators. If the list's $output is a simple import expression,
    /// it returns an ImportedGenerator value; otherwise it evaluates to Text.
    ///
    /// # Arguments
    ///
    /// * `list` - The compiled list to evaluate
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The evaluated value (either ImportedGenerator or Text)
    /// * `Err(EvalError)` - An error if evaluation fails
    pub(super) async fn evaluate_list_to_value(
        &mut self,
        list: &CompiledList,
    ) -> Result<Value, EvalError> {
        // Check if list has $output property
        if let Some(output_content) = &list.output {
            // Check if the output is a simple import expression
            if output_content.len() == 1 {
                if let ContentPart::Inline(inline_spanned) = &output_content[0].value {
                    let inline = &inline_spanned.value;
                    if inline.choices.len() == 1 && inline.choices[0].value.content.len() == 1 {
                        if let ContentPart::Reference(expr_spanned) =
                            &inline.choices[0].value.content[0].value
                        {
                            if let Expression::Import(name) = &expr_spanned.value {
                                // Load the import to ensure it's cached
                                let _ = self.load_import(name, Span::dummy()).await?;
                                return Ok(Value::ImportedGenerator(name.clone()));
                            }
                        }
                    }
                }
            }
        }

        // Default: evaluate as text and return as Text value
        let result = self.evaluate_list(list, None).await?;
        Ok(Value::Text(result))
    }
}
