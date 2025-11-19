//! Expression evaluation methods for the Evaluator
//!
//! This module contains the core expression evaluation logic:
//! - `evaluate_expression` - Main expression evaluator that produces string output
//! - `evaluate_to_value` - Expression evaluator that produces Value enum output
//! - `value_to_string` - Converts Value enum instances to string output
//!
//! These methods handle all expression types including:
//! - Simple identifiers and list references
//! - Property access (base.property)
//! - Method calls (base.method(args))
//! - Dynamic property access (base[expr])
//! - Variable assignment (x = value)
//! - Property assignment (this.property = value)
//! - Binary operations (+, -, *, /, ==, <, >, etc.)
//! - Conditionals (condition ? true : false, if/else)
//! - Imports ({import:generator})
//! - Number and letter ranges ({1-10}, {a-z})
//! - Repeat loops ({repeat N, expression})

use crate::ast::*;
use crate::span::{Span, Spanned};
use crate::trace::OperationType;
use async_recursion::async_recursion;
use rand::Rng;
use std::sync::Arc;

use super::value::{ConsumableListState, Value};
use super::{EvalError, Evaluator};

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    /// Evaluate an expression and return its string representation
    ///
    /// This is the main expression evaluator that handles all expression types
    /// and produces string output. It recursively evaluates nested expressions
    /// and handles special cases like the "this" keyword and import expressions.
    ///
    /// # Arguments
    ///
    /// * `expr_spanned` - The spanned expression to evaluate
    ///
    /// # Returns
    ///
    /// The string result of evaluating the expression, or an error if evaluation fails
    #[async_recursion]
    pub(super) async fn evaluate_expression(
        &mut self,
        expr_spanned: &Spanned<Expression>,
    ) -> Result<String, EvalError> {
        let expr = &expr_spanned.value;
        let expr_span = expr_spanned.span;

        match expr {
            Expression::Simple(ident_spanned) => {
                let ident = &ident_spanned.value;
                let span = ident_spanned.span;

                // Check for "this" keyword
                if ident.name == "this" {
                    return Err(EvalError::TypeError {
                        message: "Cannot use 'this' without property access (use this.property)"
                            .to_string(),
                        span,
                    });
                }

                // Check if it's a variable first
                if let Some(value) = self.variables.get(&ident.name) {
                    return self.value_to_string(value.clone()).await;
                }

                // Otherwise, look up the list and evaluate it
                match self.program.get_list(&ident.name) {
                    Some(list) => self.evaluate_list(list, Some(span)).await,
                    None => Err(EvalError::UndefinedList {
                        name: ident.name.clone(),
                        span,
                    }),
                }
            }

            Expression::Property(base_spanned, prop_spanned) => {
                let prop = &prop_spanned.value;
                let span = expr_span;

                // Special handling for "this" keyword
                if let Expression::Simple(ident_spanned) = &base_spanned.value {
                    if ident_spanned.value.name == "this" {
                        if self.current_item.is_none() {
                            return Err(EvalError::TypeError {
                                message: "'this' keyword can only be used within $output"
                                    .to_string(),
                                span,
                            });
                        }

                        // Check dynamic_properties first (for assigned properties)
                        if let Some(value) = self.dynamic_properties.get(&prop.name) {
                            return self.value_to_string(value.clone()).await;
                        }

                        // Then check the current_item's sublists
                        if let Some(ref item) = self.current_item {
                            // Direct property access
                            if let Some(sublist) = item.sublists.get(&prop.name) {
                                let sublist_clone = sublist.clone();
                                return self.evaluate_list(&sublist_clone, Some(span)).await;
                            }

                            // If the item has exactly one sublist, delegate to it
                            if item.sublists.len() == 1 {
                                let single_sublist = item.sublists.values().next().unwrap();
                                // Search through items in the single sublist for the property
                                for subitem in &single_sublist.items {
                                    if let Some(target_sublist) = subitem.sublists.get(&prop.name) {
                                        let target_clone = target_sublist.clone();
                                        return self.evaluate_list(&target_clone, Some(span)).await;
                                    }
                                }
                            }
                        }

                        // Property not found
                        return Err(EvalError::UndefinedProperty {
                            list: "this".to_string(),
                            prop: prop.name.clone(),
                            span,
                        });
                    }
                }

                let base_value = self.evaluate_to_value(base_spanned).await?;
                // Try as property first, then as a zero-argument method
                match self.get_property(&base_value, &prop.name, span).await {
                    Ok(result) => Ok(result),
                    Err(EvalError::UndefinedProperty { .. }) => {
                        // Try as a method call with no arguments
                        let method = MethodCall::new(prop.name.clone());
                        self.call_method(&base_value, &method, span).await
                    }
                    Err(e) => Err(e),
                }
            }

            Expression::PropertyWithFallback(base_spanned, prop_spanned, fallback_spanned) => {
                let prop = &prop_spanned.value;
                let span = expr_span;

                // Try to access the property, fall back to the fallback expression if it doesn't exist
                let base_value = self.evaluate_to_value(base_spanned).await?;
                match self.get_property(&base_value, &prop.name, span).await {
                    Ok(result) => Ok(result),
                    Err(EvalError::UndefinedProperty { .. }) | Err(EvalError::TypeError { .. }) => {
                        // Property doesn't exist, evaluate fallback
                        self.evaluate_expression(fallback_spanned).await
                    }
                    Err(e) => Err(e),
                }
            }

            Expression::Dynamic(base_spanned, index_spanned) => {
                let span = expr_span;
                let base_value = self.evaluate_to_value(base_spanned).await?;
                let index_str = self.evaluate_expression(index_spanned).await?;
                self.get_property(&base_value, &index_str, span).await
            }

            Expression::Method(base_spanned, method_spanned) => {
                let method = &method_spanned.value;
                let span = expr_span;

                // Check if this is a direct function call (e.g., joinLists(args))
                // where the base is a Simple identifier that matches the method name
                if let Expression::Simple(ident_spanned) = &base_spanned.value {
                    if ident_spanned.value.name == method.name && method.name == "joinLists" {
                        // Handle as a built-in function call
                        // We create a dummy value to pass to call_method_value
                        let result = self
                            .call_method_value(&Value::Text(String::new()), method, span)
                            .await?;
                        return self.value_to_string(result).await;
                    }

                    // Special handling for "this" keyword
                    if ident_spanned.value.name == "this" {
                        if let Some(ref item) = self.current_item {
                            let value = Value::ItemInstance(item.clone());
                            return self.call_method(&value, method, span).await;
                        } else {
                            return Err(EvalError::TypeError {
                                message: "'this' keyword can only be used within $output"
                                    .to_string(),
                                span,
                            });
                        }
                    }
                }

                let base_value = self.evaluate_to_value(base_spanned).await?;
                self.call_method(&base_value, method, span).await
            }

            Expression::Assignment(ident_spanned, value_spanned) => {
                let ident = &ident_spanned.value;
                let mut val = self.evaluate_to_value(value_spanned).await?;

                // If assigning a list reference (but not a consumable list), select one item from it
                // This ensures subsequent uses of the variable get the same selection
                // Consumable lists and ListInstances are excluded because they have their own selection logic
                if matches!(val, Value::List(_)) {
                    let method = MethodCall::new("selectOne".to_string());
                    val = self.call_method_value(&val, &method, Span::dummy()).await?;
                }

                self.variables.insert(ident.name.clone(), val.clone());
                // Assignments return the assigned value
                self.value_to_string(val).await
            }

            Expression::Sequence(exprs, output) => {
                // Evaluate all expressions in sequence
                for expr_sp in exprs {
                    self.evaluate_expression(expr_sp).await?;
                }

                // Return the output expression if present
                if let Some(out_expr_sp) = output {
                    self.evaluate_expression(out_expr_sp).await
                } else {
                    Ok(String::new())
                }
            }

            Expression::Literal(s) => {
                // Evaluate the literal string (it may contain references)
                // We need to parse and evaluate the string content
                // For now, we'll use a simple approach: re-parse the string as content
                match crate::parser::Parser::new(s).parse_content_until_newline() {
                    Ok(content) => self.evaluate_content(&content).await,
                    Err(_) => Ok(s.clone()), // Fallback to literal if parsing fails
                }
            }

            Expression::Number(n) => Ok(self.format_number(*n)),

            Expression::PropertyAssignment(base_spanned, prop_spanned, value_spanned) => {
                let prop = &prop_spanned.value;
                let span = expr_span;

                // Handle property assignment like [this.property = value]
                // Currently only supported for 'this' keyword
                if let Expression::Simple(ident_spanned) = &base_spanned.value {
                    if ident_spanned.value.name == "this" {
                        if self.current_item.is_none() {
                            return Err(EvalError::TypeError {
                                message: "'this' keyword can only be used within $output"
                                    .to_string(),
                                span,
                            });
                        }

                        // Evaluate the value and store it
                        let val_str = self.evaluate_expression(value_spanned).await?;
                        let val = Value::Text(val_str.clone());

                        // Store in dynamic_properties for later access
                        self.dynamic_properties.insert(prop.name.clone(), val);

                        // Return the assigned value
                        return Ok(val_str);
                    }
                }

                Err(EvalError::TypeError {
                    message: "Property assignment is only supported for 'this' keyword".to_string(),
                    span,
                })
            }

            Expression::NumberRange(start, end) => {
                let num = self.rng.gen_range(*start..=*end);
                Ok(num.to_string())
            }

            Expression::LetterRange(start, end) => {
                let start_byte = *start as u8;
                let end_byte = *end as u8;
                let random_byte = self.rng.gen_range(start_byte..=end_byte);
                Ok((random_byte as char).to_string())
            }

            Expression::Conditional(cond_spanned, true_spanned, false_spanned) => {
                // Evaluate condition
                let cond_result = self.evaluate_expression(cond_spanned).await?;

                // Check if condition is truthy
                if self.is_truthy(&cond_result) {
                    self.evaluate_expression(true_spanned).await
                } else {
                    self.evaluate_expression(false_spanned).await
                }
            }

            Expression::IfElse {
                condition,
                then_expr,
                else_expr,
            } => {
                // Evaluate condition
                let cond_result = self.evaluate_expression(condition).await?;

                // Check if condition is truthy
                if self.is_truthy(&cond_result) {
                    self.evaluate_expression(then_expr).await
                } else if let Some(else_branch) = else_expr {
                    self.evaluate_expression(else_branch).await
                } else {
                    // No else branch, return empty string
                    Ok(String::new())
                }
            }

            Expression::Repeat { count, body } => {
                let span = expr_span;
                // Evaluate the count expression
                let count_str = self.evaluate_expression(count).await?;
                let n = count_str
                    .parse::<usize>()
                    .map_err(|_| EvalError::TypeError {
                        message: format!("repeat count must be a number, got: {}", count_str),
                        span,
                    })?;

                // Evaluate the body n times and concatenate results
                let mut result = String::new();
                for _ in 0..n {
                    let iteration_result = self.evaluate_expression(body).await?;
                    result.push_str(&iteration_result);
                }

                Ok(result)
            }

            Expression::BinaryOp(left_spanned, op, right_spanned) => {
                use BinaryOperator::*;
                let span = expr_span;

                match op {
                    // Math operations
                    Add | Subtract | Multiply | Divide | Modulo => {
                        let left_val = self.evaluate_expression(left_spanned).await?;
                        let right_val = self.evaluate_expression(right_spanned).await?;

                        // For addition, check if either value is non-numeric (string concatenation)
                        if matches!(op, Add) {
                            let left_num = left_val.parse::<f64>();
                            let right_num = right_val.parse::<f64>();

                            if left_num.is_ok() && right_num.is_ok() {
                                // Both are numbers, do numeric addition
                                let result = left_num.unwrap() + right_num.unwrap();
                                Ok(self.format_number(result))
                            } else {
                                // String concatenation
                                Ok(format!("{}{}", left_val, right_val))
                            }
                        } else {
                            // Other math operations require numbers
                            let left_num =
                                left_val.parse::<f64>().map_err(|_| EvalError::TypeError {
                                    message: format!("Left operand is not a number: {}", left_val),
                                    span,
                                })?;
                            let right_num =
                                right_val.parse::<f64>().map_err(|_| EvalError::TypeError {
                                    message: format!(
                                        "Right operand is not a number: {}",
                                        right_val
                                    ),
                                    span,
                                })?;

                            let result = match op {
                                Subtract => left_num - right_num,
                                Multiply => left_num * right_num,
                                Divide => {
                                    if right_num == 0.0 {
                                        return Err(EvalError::TypeError {
                                            message: "Division by zero".to_string(),
                                            span,
                                        });
                                    }
                                    left_num / right_num
                                }
                                Modulo => {
                                    if right_num == 0.0 {
                                        return Err(EvalError::TypeError {
                                            message: "Modulo by zero".to_string(),
                                            span,
                                        });
                                    }
                                    left_num % right_num
                                }
                                _ => unreachable!(),
                            };

                            Ok(self.format_number(result))
                        }
                    }

                    // Comparison and logical operations
                    _ => {
                        let left_val = self.evaluate_expression(left_spanned).await?;
                        let right_val = self.evaluate_expression(right_spanned).await?;

                        let result = match op {
                            Equal => left_val == right_val,
                            NotEqual => left_val != right_val,
                            LessThan => self.compare_values(&left_val, &right_val)? < 0,
                            GreaterThan => self.compare_values(&left_val, &right_val)? > 0,
                            LessEqual => self.compare_values(&left_val, &right_val)? <= 0,
                            GreaterEqual => self.compare_values(&left_val, &right_val)? >= 0,
                            And => self.is_truthy(&left_val) && self.is_truthy(&right_val),
                            Or => self.is_truthy(&left_val) || self.is_truthy(&right_val),
                            _ => unreachable!(),
                        };

                        Ok(if result { "true" } else { "false" }.to_string())
                    }
                }
            }

            Expression::Import(generator_name) => {
                let span = expr_span;

                // Start trace for import
                self.trace_start(
                    format!("{{import:{}}}", generator_name),
                    OperationType::Import,
                    Some(span),
                );

                // Load the imported generator
                let imported_program = self.load_import(generator_name, span).await?.clone();

                // Create a new evaluator for the imported program with its own context
                let mut imported_evaluator = Evaluator::new(&imported_program, self.rng);

                // Copy the loader reference so nested imports work
                if let Some(ref loader) = self.loader {
                    imported_evaluator.loader = Some(Arc::clone(loader));
                }

                // Share the import cache and sources
                imported_evaluator.import_cache = self.import_cache.clone();
                imported_evaluator.import_sources = self.import_sources.clone();

                // Enable tracing for the imported evaluator if we're tracing
                if self.trace_enabled {
                    imported_evaluator = imported_evaluator.with_tracing();

                    // Set source template and generator name for all child nodes
                    if let Some(source) = self.import_sources.get(generator_name) {
                        imported_evaluator =
                            imported_evaluator.with_source(source.clone(), generator_name.clone());
                    }
                }

                // Evaluate the imported generator
                let result = imported_evaluator.evaluate().await;

                // If we're tracing, add the imported trace as a child
                if let Ok(ref output) = result {
                    if self.trace_enabled {
                        if let Some(mut imported_trace) = imported_evaluator.take_trace() {
                            // Add source template and generator name to the imported trace
                            if let Some(source) = self.import_sources.get(generator_name) {
                                imported_trace.source_template = Some(source.clone());
                                imported_trace.generator_name = Some(generator_name.clone());
                            }

                            if let Some(node) = self.trace_stack.last_mut() {
                                node.add_child(imported_trace);
                            }
                        }
                    }
                    self.trace_end(output.clone());
                }

                result
            }
        }
    }

    /// Evaluate an expression and return its Value representation
    ///
    /// This evaluator produces Value enum instances rather than strings,
    /// which is useful for property access and method calls that need to
    /// work with structured values.
    ///
    /// # Arguments
    ///
    /// * `expr_spanned` - The spanned expression to evaluate
    ///
    /// # Returns
    ///
    /// A Value representing the expression result, or an error if evaluation fails
    #[async_recursion]
    pub(super) async fn evaluate_to_value(
        &mut self,
        expr_spanned: &Spanned<Expression>,
    ) -> Result<Value, EvalError> {
        let expr = &expr_spanned.value;
        let expr_span = expr_spanned.span;

        match expr {
            Expression::Simple(ident_spanned) => {
                let ident = &ident_spanned.value;
                let span = ident_spanned.span;

                // Handle "this" keyword
                if ident.name == "this" {
                    return Err(EvalError::TypeError {
                        message: "Cannot use 'this' without property access (use this.property)"
                            .to_string(),
                        span,
                    });
                }

                // Check variables first
                if let Some(value) = self.variables.get(&ident.name) {
                    return Ok(value.clone());
                }

                // Check if it's a list reference
                if self.program.get_list(&ident.name).is_some() {
                    return Ok(Value::List(ident.name.clone()));
                }

                Err(EvalError::UndefinedList {
                    name: ident.name.clone(),
                    span,
                })
            }

            Expression::Property(base_spanned, prop_spanned) => {
                let prop = &prop_spanned.value;
                let span = expr_span;

                // Special handling for "this" keyword
                if let Expression::Simple(ident_spanned) = &base_spanned.value {
                    if ident_spanned.value.name == "this" {
                        // Access property from current_item
                        if let Some(ref item) = self.current_item {
                            if let Some(sublist) = item.sublists.get(&prop.name) {
                                return Ok(Value::ListInstance(sublist.clone()));
                            } else {
                                return Err(EvalError::UndefinedProperty {
                                    list: "this".to_string(),
                                    prop: prop.name.clone(),
                                    span,
                                });
                            }
                        } else {
                            return Err(EvalError::TypeError {
                                message: "'this' keyword can only be used within $output"
                                    .to_string(),
                                span,
                            });
                        }
                    }
                }

                let base_value = self.evaluate_to_value(base_spanned).await?;
                // Try as property first, then as a zero-argument method
                match self.get_property_value(&base_value, &prop.name, span).await {
                    Ok(value) => Ok(value),
                    Err(EvalError::UndefinedProperty { .. }) | Err(EvalError::TypeError { .. }) => {
                        // Try as a method call with no arguments
                        let method = MethodCall::new(prop.name.clone());
                        self.call_method_value(&base_value, &method, span).await
                    }
                    Err(e) => Err(e),
                }
            }

            Expression::PropertyWithFallback(base_spanned, prop_spanned, fallback_spanned) => {
                let prop = &prop_spanned.value;
                let span = expr_span;

                // Try to access the property, fall back to the fallback expression if it doesn't exist
                let base_value = self.evaluate_to_value(base_spanned).await?;
                match self.get_property_value(&base_value, &prop.name, span).await {
                    Ok(value) => Ok(value),
                    Err(EvalError::UndefinedProperty { .. }) | Err(EvalError::TypeError { .. }) => {
                        // Property doesn't exist, evaluate fallback
                        self.evaluate_to_value(fallback_spanned).await
                    }
                    Err(e) => Err(e),
                }
            }

            Expression::Method(base_spanned, method_spanned) => {
                let method = &method_spanned.value;
                let span = method_spanned.span;

                // Check if this is a direct function call (e.g., joinLists(args))
                if let Expression::Simple(ident_spanned) = &base_spanned.value {
                    if ident_spanned.value.name == method.name && method.name == "joinLists" {
                        // Handle as a built-in function call
                        return self
                            .call_method_value(&Value::Text(String::new()), method, span)
                            .await;
                    }
                }

                let base_value = self.evaluate_to_value(base_spanned).await?;
                self.call_method_value(&base_value, method, span).await
            }

            Expression::Import(generator_name) => {
                let span = expr_span;
                // Load the imported generator to ensure it exists and is cached
                let _ = self.load_import(generator_name, span).await?;
                // Return a reference to the imported generator
                Ok(Value::ImportedGenerator(generator_name.clone()))
            }

            _ => {
                let result = self.evaluate_expression(expr_spanned).await?;
                Ok(Value::Text(result))
            }
        }
    }

    /// Convert a Value to its string representation
    ///
    /// Handles all Value enum variants and produces appropriate string output.
    /// For lists and items, triggers evaluation. For arrays, joins with spaces.
    /// For consumable lists, selects and removes one item.
    ///
    /// # Arguments
    ///
    /// * `value` - The Value to convert to a string
    ///
    /// # Returns
    ///
    /// The string representation of the value, or an error if conversion fails
    #[async_recursion]
    pub(super) async fn value_to_string(&mut self, value: Value) -> Result<String, EvalError> {
        match value {
            Value::Text(s) => Ok(s),
            Value::List(name) => {
                let list =
                    self.program
                        .get_list(&name)
                        .ok_or_else(|| EvalError::UndefinedList {
                            name: name.clone(),
                            span: Span::dummy(),
                        })?;
                self.evaluate_list(list, None).await
            }
            Value::ListInstance(list) => self.evaluate_list(&list, None).await,
            Value::ItemInstance(item) => {
                // Evaluate the item's content
                // If it has sublists, pick one randomly
                if !item.sublists.is_empty() {
                    let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                    let idx = self.rng.gen_range(0..sublist_names.len());
                    let sublist_name = &sublist_names[idx];
                    let sublist = item.sublists.get(sublist_name).unwrap();
                    self.evaluate_list(sublist, None).await
                } else {
                    self.evaluate_content(&item.content).await
                }
            }
            Value::Array(items) => Ok(items.join(" ")), // Default: space-separated
            Value::ConsumableList(id) => {
                // Get the consumable list state
                let state =
                    self.consumable_lists
                        .get(&id)
                        .ok_or_else(|| EvalError::UndefinedList {
                            name: format!("Consumable list not found: {}", id),
                            span: Span::dummy(),
                        })?;

                // Check if there are any items left
                if state.remaining_indices.is_empty() {
                    return Err(EvalError::EmptyList {
                        name: format!(
                            "Consumable list '{}' has been exhausted",
                            state.source_list.name
                        ),
                        span: Span::dummy(),
                    });
                }

                // Clone the source list and remaining indices before selecting
                let source_list = state.source_list.clone();
                let remaining_indices = state.remaining_indices.clone();

                // Select a random index from remaining_indices
                let idx = self.rng.gen_range(0..remaining_indices.len());
                let item_idx = remaining_indices[idx];

                // Get and clone the item
                let item = source_list
                    .items
                    .get(item_idx)
                    .ok_or_else(|| EvalError::EmptyList {
                        name: format!("Invalid index {} in consumable list", item_idx),
                        span: Span::dummy(),
                    })?
                    .clone();

                // Remove the selected index from remaining_indices
                let mut new_remaining = remaining_indices;
                new_remaining.remove(idx);

                // Update the consumable list state
                self.consumable_lists.insert(
                    id.clone(),
                    ConsumableListState {
                        source_list,
                        remaining_indices: new_remaining,
                    },
                );

                // Evaluate the item
                if !item.sublists.is_empty() {
                    let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                    let sidx = self.rng.gen_range(0..sublist_names.len());
                    let sublist_name = &sublist_names[sidx];
                    let sublist = item.sublists.get(sublist_name).unwrap();
                    self.evaluate_list(sublist, None).await
                } else {
                    self.evaluate_content(&item.content).await
                }
            }
            Value::ImportedGenerator(generator_name) => {
                // Evaluate the imported generator
                let imported_program = self
                    .load_import(&generator_name, Span::dummy())
                    .await?
                    .clone();

                // Create a new evaluator for the imported program with its own context
                let mut imported_evaluator = Evaluator::new(&imported_program, self.rng);

                // Copy the loader reference so nested imports work
                if let Some(ref loader) = self.loader {
                    imported_evaluator.loader = Some(Arc::clone(loader));
                }

                // Share the import cache
                imported_evaluator.import_cache = self.import_cache.clone();

                // Evaluate the imported generator
                imported_evaluator.evaluate().await
            }
        }
    }
}
