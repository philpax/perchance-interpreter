//! Property access and method call implementations for the Evaluator
//!
//! This module contains all property-related operations:
//! - `get_property_value` - Resolves property access on values (returns Value)
//! - `get_property` - Resolves property access and converts to string
//! - `is_grammar_method` - Checks if a name is a grammar transformation method
//! - `extract_simple_list_reference` - Extracts list names from simple references
//! - `call_method` - Calls a method and converts result to string
//! - `call_method_value` - Calls a method and returns Value
//!
//! Supported methods include:
//! - Selection: selectOne, selectAll, selectMany, selectUnique
//! - Grammar: pluralForm, singularForm, upperCase, lowerCase, titleCase, sentenceCase,
//!           pastTense, presentTense, futureTense, negativeForm, possessiveForm
//! - List operations: consumableList, joinLists, joinItems
//! - Item operations: evaluateItem

use super::grammar::*;
use super::value::{ConsumableListState, Value};
use crate::ast::*;
use crate::compiler::*;
use crate::span::{Span, Spanned};
use async_recursion::async_recursion;
use rand::Rng;

impl<'a, R: Rng + Send> super::Evaluator<'a, R> {
    #[async_recursion]
    pub(super) async fn get_property_value(
        &mut self,
        value: &Value,
        prop_name: &str,
        span: Span,
    ) -> Result<Value, super::EvalError> {
        match value {
            Value::List(list_name) => {
                // Look up the list
                let list = self.program.get_list(list_name).ok_or_else(|| {
                    super::EvalError::UndefinedList {
                        name: list_name.clone(),
                        span,
                    }
                })?;

                // Search through all items to find one with this property as a sublist
                for item in &list.items {
                    if let Some(sublist) = item.sublists.get(prop_name) {
                        return Ok(Value::ListInstance(sublist.clone()));
                    }
                }

                // If no items have the property, check if the list has a $output
                // that evaluates to an imported generator
                if list.items.is_empty() && list.output.is_some() {
                    // Try to evaluate the list to see if it produces an ImportedGenerator
                    let result_value = self.evaluate_list_to_value(list).await?;
                    if matches!(result_value, Value::ImportedGenerator(_)) {
                        // Delegate property access to the imported generator
                        return self
                            .get_property_value(&result_value, prop_name, span)
                            .await;
                    }
                }

                Err(super::EvalError::UndefinedProperty {
                    list: list_name.clone(),
                    prop: prop_name.to_string(),
                    span,
                })
            }
            Value::ListInstance(list) => {
                // Search through all items to find one with this property
                for item in &list.items {
                    if let Some(sublist) = item.sublists.get(prop_name) {
                        return Ok(Value::ListInstance(sublist.clone()));
                    }
                }

                Err(super::EvalError::UndefinedProperty {
                    list: list.name.clone(),
                    prop: prop_name.to_string(),
                    span,
                })
            }
            Value::ItemInstance(item) => {
                // Access a property (sublist) of an item
                if let Some(sublist) = item.sublists.get(prop_name) {
                    Ok(Value::ListInstance(sublist.clone()))
                } else if item.sublists.len() == 1 {
                    // If the item has exactly one sublist, try to access the property from that sublist
                    let single_sublist = item.sublists.values().next().unwrap();
                    // Search through items in the single sublist for the property
                    for subitem in &single_sublist.items {
                        if let Some(target_sublist) = subitem.sublists.get(prop_name) {
                            return Ok(Value::ListInstance(target_sublist.clone()));
                        }
                    }
                    Err(super::EvalError::UndefinedProperty {
                        list: "item".to_string(),
                        prop: prop_name.to_string(),
                        span,
                    })
                } else {
                    Err(super::EvalError::UndefinedProperty {
                        list: "item".to_string(),
                        prop: prop_name.to_string(),
                        span,
                    })
                }
            }
            Value::Text(_) => {
                // Check if this is a grammar method that can be applied to text
                if self.is_grammar_method(prop_name) {
                    let method = MethodCall::new(prop_name.to_string());
                    return self.call_method_value(value, &method, span).await;
                }
                Err(super::EvalError::TypeError {
                    message: format!("Cannot access property '{}' on text value", prop_name),
                    span,
                })
            }
            Value::Array(_) => Err(super::EvalError::TypeError {
                message: format!("Cannot access property '{}' on array value", prop_name),
                span,
            }),
            Value::ConsumableList(_) => {
                // Check if this is a method that can be applied to consumable lists
                if self.is_grammar_method(prop_name) || prop_name == "selectOne" {
                    let method = MethodCall::new(prop_name.to_string());
                    return self.call_method_value(value, &method, span).await;
                }
                Err(super::EvalError::TypeError {
                    message: format!("Cannot access property '{}' on consumable list", prop_name),
                    span,
                })
            }
            Value::ImportedGenerator(generator_name) => {
                // Access a property (top-level list) from the imported generator
                let imported_program = self.load_import(generator_name, span).await?;

                // Look up the list by name in the imported generator
                if let Some(list) = imported_program.get_list(prop_name) {
                    Ok(Value::ListInstance(list.clone()))
                } else {
                    Err(super::EvalError::UndefinedProperty {
                        list: generator_name.clone(),
                        prop: prop_name.to_string(),
                        span,
                    })
                }
            }
        }
    }

    pub(super) fn is_grammar_method(&self, name: &str) -> bool {
        matches!(
            name,
            "pluralForm"
                | "singularForm"
                | "upperCase"
                | "lowerCase"
                | "titleCase"
                | "sentenceCase"
                | "pastTense"
                | "presentTense"
                | "futureTense"
                | "negativeForm"
                | "possessiveForm"
        )
    }

    /// Helper to extract a simple list reference from content like [listname]
    /// Returns the list name if the content is a simple reference, None otherwise
    pub(super) fn extract_simple_list_reference(
        content: &[Spanned<ContentPart>],
    ) -> Option<String> {
        // Check if content is exactly one reference
        if content.len() == 1 {
            // Case 1: Direct reference like in $output = [color]
            if let ContentPart::Reference(expr_spanned) = &content[0].value {
                if let Expression::Simple(ident_spanned) = &expr_spanned.value {
                    return Some(ident_spanned.value.name.clone());
                }
            }
            // Case 2: Inline expression with one choice containing one reference
            if let ContentPart::Inline(inline_spanned) = &content[0].value {
                let inline = &inline_spanned.value;
                if inline.choices.len() == 1 && inline.choices[0].value.content.len() == 1 {
                    if let ContentPart::Reference(expr_spanned) =
                        &inline.choices[0].value.content[0].value
                    {
                        if let Expression::Simple(ident_spanned) = &expr_spanned.value {
                            return Some(ident_spanned.value.name.clone());
                        }
                    }
                }
            }
        }
        None
    }

    #[async_recursion]
    pub(super) async fn get_property(
        &mut self,
        value: &Value,
        prop_name: &str,
        span: Span,
    ) -> Result<String, super::EvalError> {
        let prop_value = self.get_property_value(value, prop_name, span).await?;
        self.value_to_string(prop_value).await
    }

    #[async_recursion]
    pub(super) async fn call_method(
        &mut self,
        value: &Value,
        method: &MethodCall,
        span: Span,
    ) -> Result<String, super::EvalError> {
        let value_result = self.call_method_value(value, method, span).await?;
        self.value_to_string(value_result).await
    }

    #[async_recursion]
    pub(super) async fn call_method_value(
        &mut self,
        value: &Value,
        method: &MethodCall,
        span: Span,
    ) -> Result<Value, super::EvalError> {
        match method.name.as_str() {
            "selectOne" => {
                // Select one item from the list and return it as a Value
                match value {
                    Value::List(name) => {
                        let list = self.program.get_list(name).ok_or_else(|| {
                            super::EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            }
                        })?;

                        let (item_ref, _idx) = self
                            .select_weighted_item(&list.items, list.total_weight)
                            .await?;
                        let item = item_ref.clone();

                        // If item has sublists (properties), return the item instance
                        // so properties can be accessed later
                        if !item.sublists.is_empty() {
                            return Ok(Value::ItemInstance(item));
                        }

                        // No sublists, evaluate content directly
                        let result = self.evaluate_content(&item.content).await?;
                        Ok(Value::Text(result))
                    }
                    Value::ListInstance(list) => {
                        let (item_ref, _idx) = self
                            .select_weighted_item(&list.items, list.total_weight)
                            .await?;
                        let item = item_ref.clone();

                        // If item has sublists (properties), return the item instance
                        if !item.sublists.is_empty() {
                            return Ok(Value::ItemInstance(item));
                        }

                        let result = self.evaluate_content(&item.content).await?;
                        Ok(Value::Text(result))
                    }
                    Value::ItemInstance(item) => Ok(Value::ItemInstance(item.clone())),
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    Value::Array(items) => {
                        // Select one item from the array
                        if items.is_empty() {
                            Ok(Value::Text(String::new()))
                        } else {
                            let idx = self.rng.gen_range(0..items.len());
                            Ok(Value::Text(items[idx].clone()))
                        }
                    }
                    Value::ConsumableList(_) => {
                        // For consumable lists, convert to string (which consumes an item)
                        let result = self.value_to_string(value.clone()).await?;
                        Ok(Value::Text(result))
                    }
                    Value::ImportedGenerator(_) => {
                        // Convert imported generator to string (evaluates it)
                        let result = self.value_to_string(value.clone()).await?;
                        Ok(Value::Text(result))
                    }
                }
            }

            "evaluateItem" => {
                // Evaluate an item immediately, converting it to text
                match value {
                    Value::ItemInstance(item) => {
                        // Evaluate the item's content immediately
                        let result = self.evaluate_content(&item.content).await?;
                        Ok(Value::Text(result))
                    }
                    Value::Text(s) => {
                        // Already text, just return it
                        Ok(Value::Text(s.clone()))
                    }
                    _ => {
                        // For other values, convert to string
                        let result = self.value_to_string(value.clone()).await?;
                        Ok(Value::Text(result))
                    }
                }
            }

            "upperCase" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(s.to_uppercase()))
            }

            "lowerCase" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(s.to_lowercase()))
            }

            "titleCase" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_title_case(&s)))
            }

            "sentenceCase" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_sentence_case(&s)))
            }

            "selectAll" => {
                // Return all items as a joined string
                match value {
                    Value::List(name) => {
                        let list = self.program.get_list(name).ok_or_else(|| {
                            super::EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            }
                        })?;

                        let mut results = Vec::new();
                        for item in &list.items {
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                for sublist_name in sublist_names {
                                    if let Some(sublist) = item.sublists.get(&sublist_name) {
                                        results.push(self.evaluate_list(sublist, None).await?);
                                    }
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content).await?);
                            }
                        }
                        Ok(Value::Text(results.join(" ")))
                    }
                    Value::ListInstance(list) => {
                        let mut results = Vec::new();
                        for item in &list.items {
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                for sublist_name in sublist_names {
                                    if let Some(sublist) = item.sublists.get(&sublist_name) {
                                        results.push(self.evaluate_list(sublist, None).await?);
                                    }
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content).await?);
                            }
                        }
                        Ok(Value::Text(results.join(" ")))
                    }
                    Value::ItemInstance(item) => {
                        // For an item instance, just evaluate its content
                        let result = self.evaluate_content(&item.content).await?;
                        Ok(Value::Text(result))
                    }
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    Value::Array(items) => {
                        // selectAll on an array just returns the array
                        Ok(Value::Array(items.clone()))
                    }
                    Value::ConsumableList(_) => {
                        // selectAll is not meaningful for consumable lists
                        Err(super::EvalError::InvalidMethodCall {
                            message: "selectAll cannot be called on consumable lists".to_string(),
                            span,
                        })
                    }
                    Value::ImportedGenerator(_) => {
                        // selectAll is not meaningful for imported generators
                        Err(super::EvalError::InvalidMethodCall {
                            message: "selectAll cannot be called on imported generators"
                                .to_string(),
                            span,
                        })
                    }
                }
            }

            "selectMany" => {
                // Select n items with repetition
                // Supports selectMany(n) or selectMany(min, max)
                let n = if method.args.is_empty() {
                    return Err(super::EvalError::InvalidMethodCall {
                        message: "selectMany requires at least one argument".to_string(),
                        span,
                    });
                } else if method.args.len() == 1 {
                    // Single argument: exact count
                    let arg_str = self.evaluate_expression(&method.args[0]).await?;
                    arg_str
                        .parse::<usize>()
                        .map_err(|_| super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectMany argument must be a number, got: {}",
                                arg_str
                            ),
                            span,
                        })?
                } else if method.args.len() == 2 {
                    // Two arguments: random count between min and max
                    let min_str = self.evaluate_expression(&method.args[0]).await?;
                    let max_str = self.evaluate_expression(&method.args[1]).await?;
                    let min = min_str.parse::<usize>().map_err(|_| {
                        super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectMany min argument must be a number, got: {}",
                                min_str
                            ),
                            span,
                        }
                    })?;
                    let max = max_str.parse::<usize>().map_err(|_| {
                        super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectMany max argument must be a number, got: {}",
                                max_str
                            ),
                            span,
                        }
                    })?;
                    if min > max {
                        return Err(super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectMany min ({}) cannot be greater than max ({})",
                                min, max
                            ),
                            span,
                        });
                    }
                    self.rng.gen_range(min..=max)
                } else {
                    return Err(super::EvalError::InvalidMethodCall {
                        message: "selectMany accepts 1 or 2 arguments".to_string(),
                        span,
                    });
                };

                match value {
                    Value::List(name) => {
                        let list = self.program.get_list(name).ok_or_else(|| {
                            super::EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            }
                        })?;

                        let mut results = Vec::new();
                        for _ in 0..n {
                            let (item_ref, _idx) = self
                                .select_weighted_item(&list.items, list.total_weight)
                                .await?;
                            let item = item_ref.clone();
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let idx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[idx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist, None).await?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content).await?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ListInstance(list) => {
                        let mut results = Vec::new();
                        for _ in 0..n {
                            let (item_ref, _idx) = self
                                .select_weighted_item(&list.items, list.total_weight)
                                .await?;
                            let item = item_ref.clone();
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let idx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[idx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist, None).await?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content).await?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ItemInstance(item) => {
                        // Repeat the same item n times (convert to string)
                        let mut results = Vec::new();
                        for _ in 0..n {
                            let result = self.evaluate_content(&item.content).await?;
                            results.push(result);
                        }
                        Ok(Value::Array(results))
                    }
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    Value::Array(items) => {
                        // selectMany on an array
                        let mut results = Vec::new();
                        for _ in 0..n {
                            if !items.is_empty() {
                                let idx = self.rng.gen_range(0..items.len());
                                results.push(items[idx].clone());
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ConsumableList(_) => {
                        // selectMany with repetition doesn't make sense for consumable lists
                        Err(super::EvalError::InvalidMethodCall { message: "selectMany cannot be called on consumable lists (use selectUnique instead)".to_string(), span })
                    }
                    Value::ImportedGenerator(_) => {
                        // selectMany is not meaningful for imported generators
                        Err(super::EvalError::InvalidMethodCall {
                            message: "selectMany cannot be called on imported generators"
                                .to_string(),
                            span,
                        })
                    }
                }
            }

            "selectUnique" => {
                // Select n unique items without repetition
                // Supports selectUnique(n) or selectUnique(min, max)
                let n = if method.args.is_empty() {
                    return Err(super::EvalError::InvalidMethodCall {
                        message: "selectUnique requires at least one argument".to_string(),
                        span,
                    });
                } else if method.args.len() == 1 {
                    // Single argument: exact count
                    let arg_str = self.evaluate_expression(&method.args[0]).await?;
                    arg_str
                        .parse::<usize>()
                        .map_err(|_| super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectUnique argument must be a number, got: {}",
                                arg_str
                            ),
                            span,
                        })?
                } else if method.args.len() == 2 {
                    // Two arguments: random count between min and max
                    let min_str = self.evaluate_expression(&method.args[0]).await?;
                    let max_str = self.evaluate_expression(&method.args[1]).await?;
                    let min = min_str.parse::<usize>().map_err(|_| {
                        super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectUnique min argument must be a number, got: {}",
                                min_str
                            ),
                            span,
                        }
                    })?;
                    let max = max_str.parse::<usize>().map_err(|_| {
                        super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectUnique max argument must be a number, got: {}",
                                max_str
                            ),
                            span,
                        }
                    })?;
                    if min > max {
                        return Err(super::EvalError::InvalidMethodCall {
                            message: format!(
                                "selectUnique min ({}) cannot be greater than max ({})",
                                min, max
                            ),
                            span,
                        });
                    }
                    self.rng.gen_range(min..=max)
                } else {
                    return Err(super::EvalError::InvalidMethodCall {
                        message: "selectUnique accepts 1 or 2 arguments".to_string(),
                        span,
                    });
                };

                match value {
                    Value::List(name) => {
                        let list = self.program.get_list(name).ok_or_else(|| {
                            super::EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            }
                        })?;

                        if n > list.items.len() {
                            return Err(super::EvalError::InvalidMethodCall {
                                message: format!(
                                    "Cannot select {} unique items from list with {} items",
                                    n,
                                    list.items.len()
                                ),
                                span,
                            });
                        }

                        let mut available_indices: Vec<usize> = (0..list.items.len()).collect();
                        let mut results = Vec::new();

                        for _ in 0..n {
                            let idx = self.rng.gen_range(0..available_indices.len());
                            let item_idx = available_indices.remove(idx);
                            let item = &list.items[item_idx];

                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let sidx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[sidx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist, None).await?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content).await?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ListInstance(list) => {
                        if n > list.items.len() {
                            return Err(super::EvalError::InvalidMethodCall {
                                message: format!(
                                    "Cannot select {} unique items from list with {} items",
                                    n,
                                    list.items.len()
                                ),
                                span,
                            });
                        }

                        let mut available_indices: Vec<usize> = (0..list.items.len()).collect();
                        let mut results = Vec::new();

                        for _ in 0..n {
                            let idx = self.rng.gen_range(0..available_indices.len());
                            let item_idx = available_indices.remove(idx);
                            let item = &list.items[item_idx];

                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let sidx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[sidx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist, None).await?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content).await?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ItemInstance(item) => {
                        // Can't select unique items from a single item
                        if n > 1 {
                            return Err(super::EvalError::InvalidMethodCall {
                                message: "Cannot select multiple unique items from a single item"
                                    .to_string(),
                                span,
                            });
                        }
                        let result = self.evaluate_content(&item.content).await?;
                        Ok(Value::Array(vec![result]))
                    }
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    Value::Array(items) => {
                        // selectUnique on an array
                        if n > items.len() {
                            return Err(super::EvalError::InvalidMethodCall {
                                message: format!(
                                    "Cannot select {} unique items from array with {} items",
                                    n,
                                    items.len()
                                ),
                                span,
                            });
                        }

                        let mut available_indices: Vec<usize> = (0..items.len()).collect();
                        let mut results = Vec::new();

                        for _ in 0..n {
                            let idx = self.rng.gen_range(0..available_indices.len());
                            let item_idx = available_indices.remove(idx);
                            results.push(items[item_idx].clone());
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ConsumableList(_id) => {
                        // For consumable lists, consume n items
                        let mut results = Vec::new();
                        for _ in 0..n {
                            let result = self.value_to_string(value.clone()).await?;
                            results.push(result);
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ImportedGenerator(_) => {
                        // selectUnique is not meaningful for imported generators
                        Err(super::EvalError::InvalidMethodCall {
                            message: "selectUnique cannot be called on imported generators"
                                .to_string(),
                            span,
                        })
                    }
                }
            }

            "pluralForm" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_plural(&s)))
            }

            "pastTense" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_past_tense(&s)))
            }

            "possessiveForm" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_possessive(&s)))
            }

            "futureTense" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_future_tense(&s)))
            }

            "presentTense" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_present_tense(&s)))
            }

            "negativeForm" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_negative_form(&s)))
            }

            "singularForm" => {
                let s = self.value_to_string(value.clone()).await?;
                Ok(Value::Text(to_singular(&s)))
            }

            "joinItems" => {
                // Join array items with a separator
                let separator = if method.args.is_empty() {
                    " ".to_string() // Default separator
                } else {
                    self.evaluate_expression(&method.args[0]).await?
                };

                match value {
                    Value::Array(items) => Ok(Value::Text(items.join(&separator))),
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    _ => {
                        // Try converting to string first
                        let s = self.value_to_string(value.clone()).await?;
                        Ok(Value::Text(s))
                    }
                }
            }

            "consumableList" => {
                // Create a consumable copy of the list
                match value {
                    Value::List(name) => {
                        let list = self
                            .program
                            .get_list(name)
                            .ok_or_else(|| super::EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            })?
                            .clone();

                        // Check if the list has $output that evaluates to an ImportedGenerator
                        // If so, delegate to the ImportedGenerator instead
                        if list.items.is_empty() && list.output.is_some() {
                            let result_value = self.evaluate_list_to_value(&list).await?;
                            if matches!(result_value, Value::ImportedGenerator(_)) {
                                // Recursively call consumableList on the ImportedGenerator
                                return self.call_method_value(&result_value, method, span).await;
                            }
                        }

                        // Generate unique ID for this consumable list
                        let id = format!("__consumable_{}__", self.consumable_list_counter);
                        self.consumable_list_counter += 1;

                        // Create list of all item indices
                        let remaining_indices: Vec<usize> = (0..list.items.len()).collect();

                        // Store the consumable list state
                        self.consumable_lists.insert(
                            id.clone(),
                            ConsumableListState {
                                source_list: list,
                                remaining_indices,
                            },
                        );

                        // Return reference to consumable list
                        Ok(Value::ConsumableList(id))
                    }
                    Value::ListInstance(list) => {
                        // Create a consumable version from the list instance
                        let list_clone = list.clone();

                        // Generate unique ID for this consumable list
                        let id = format!("__consumable_{}__", self.consumable_list_counter);
                        self.consumable_list_counter += 1;

                        // Create list of all item indices
                        let remaining_indices: Vec<usize> = (0..list_clone.items.len()).collect();

                        // Store the consumable list state
                        self.consumable_lists.insert(
                            id.clone(),
                            ConsumableListState {
                                source_list: list_clone,
                                remaining_indices,
                            },
                        );

                        // Return reference to consumable list
                        Ok(Value::ConsumableList(id))
                    }
                    Value::ImportedGenerator(generator_name) => {
                        // For imported generators, get the output list and create a consumable version
                        let imported_program = self.load_import(generator_name, span).await?;

                        // Find the output list (check $output, then output, then last list)
                        let mut output_list = if let Some(list) =
                            imported_program.get_list("$output")
                        {
                            list.clone()
                        } else if let Some(list) = imported_program.get_list("output") {
                            list.clone()
                        } else if let Some(last_list_name) = imported_program.list_order.last() {
                            imported_program
                                .get_list(last_list_name)
                                .ok_or_else(|| super::EvalError::ImportError {
                                    message: format!(
                                        "Cannot find output list in imported generator '{}'",
                                        generator_name
                                    ),
                                    span,
                                })?
                                .clone()
                        } else {
                            return Err(super::EvalError::ImportError {
                                message: format!(
                                    "Imported generator '{}' has no lists",
                                    generator_name
                                ),
                                span,
                            });
                        };

                        // If the output list has no items but has an output property,
                        // we need to resolve it to get the actual source list
                        // This handles cases like: $output = [color]
                        while output_list.items.is_empty() && output_list.output.is_some() {
                            // Try to find the referenced list
                            // Parse the output to find list references
                            let output_content = output_list.output.as_ref().unwrap();
                            let referenced_list_name =
                                Self::extract_simple_list_reference(output_content);

                            if let Some(ref_name) = referenced_list_name {
                                if let Some(list) = imported_program.get_list(&ref_name) {
                                    output_list = list.clone();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        // Generate unique ID for this consumable list
                        let id = format!("__consumable_{}__", self.consumable_list_counter);
                        self.consumable_list_counter += 1;

                        // Create list of all item indices
                        let remaining_indices: Vec<usize> = (0..output_list.items.len()).collect();

                        // Store the consumable list state
                        self.consumable_lists.insert(
                            id.clone(),
                            ConsumableListState {
                                source_list: output_list,
                                remaining_indices,
                            },
                        );

                        // Return reference to consumable list
                        Ok(Value::ConsumableList(id))
                    }
                    _ => Err(super::EvalError::InvalidMethodCall {
                        message: "consumableList can only be called on lists".to_string(),
                        span,
                    }),
                }
            }

            "joinLists" => {
                // Join multiple lists into a single list
                // This is a built-in function that mimics the join-lists-plugin

                if method.args.is_empty() {
                    return Err(super::EvalError::InvalidMethodCall {
                        message: "joinLists requires at least one argument".to_string(),
                        span,
                    });
                }

                // Collect all items from all list arguments
                let mut combined_items = Vec::new();
                let mut list_names = Vec::new();

                for arg in &method.args {
                    // Evaluate the argument to get a list value
                    let list_value = self.evaluate_to_value(arg).await?;

                    // Get the items from the list
                    match list_value {
                        Value::List(name) => {
                            let list = self.program.get_list(&name).ok_or_else(|| {
                                super::EvalError::UndefinedList {
                                    name: name.clone(),
                                    span,
                                }
                            })?;
                            combined_items.extend(list.items.clone());
                            list_names.push(name.clone());
                        }
                        Value::ListInstance(list) => {
                            combined_items.extend(list.items.clone());
                            list_names.push(list.name.clone());
                        }
                        Value::Text(_)
                        | Value::Array(_)
                        | Value::ItemInstance(_)
                        | Value::ConsumableList(_)
                        | Value::ImportedGenerator(_) => {
                            return Err(super::EvalError::TypeError {
                                message: format!(
                                    "joinLists arguments must be lists, got {:?}",
                                    list_value
                                ),
                                span,
                            });
                        }
                    }
                }

                // Create a descriptive name showing what was joined
                let joined_name = if list_names.is_empty() {
                    "joined[]".to_string()
                } else {
                    format!("joined[{}]", list_names.join(", "))
                };

                // Create a new list with all combined items
                let combined_list = CompiledList {
                    name: joined_name,
                    items: combined_items.clone(),
                    total_weight: combined_items.iter().map(|item| item.weight).sum(),
                    output: None,
                };

                Ok(Value::ListInstance(combined_list))
            }

            _ => Err(super::EvalError::InvalidMethodCall {
                message: format!("Unknown method: {}", method.name),
                span,
            }),
        }
    }
}
