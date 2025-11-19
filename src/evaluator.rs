/// Evaluator executes compiled programs with RNG support
use crate::ast::*;
use crate::compiler::*;
use crate::loader::GeneratorLoader;
use crate::span::{Span, Spanned};
use async_recursion::async_recursion;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

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

#[derive(Debug, Clone)]
struct ConsumableListState {
    source_list: CompiledList,
    remaining_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
enum Value {
    Text(String),
    List(String),               // Reference to a list by name
    ListInstance(CompiledList), // An actual list instance (for sublists)
    ItemInstance(CompiledItem), // An item with its properties (sublists) intact
    Array(Vec<String>),         // Multiple items (for selectMany/selectUnique before joinItems)
    ConsumableList(String),     // Reference to a consumable list by unique ID
    ImportedGenerator(String),  // Reference to an imported generator by name
}

pub struct Evaluator<'a, R: Rng> {
    program: &'a CompiledProgram,
    rng: &'a mut R,
    variables: HashMap<String, Value>,
    last_number: Option<i64>, // Track last number for {s} pluralization
    current_item: Option<CompiledItem>, // Track current item for `this` keyword
    dynamic_properties: HashMap<String, Value>, // Track dynamic properties assigned via `this.property = value`
    consumable_lists: HashMap<String, ConsumableListState>, // Track consumable lists
    consumable_list_counter: usize,             // Counter for generating unique IDs
    loader: Option<Arc<dyn GeneratorLoader>>,   // Generator loader for imports
    import_cache: HashMap<String, CompiledProgram>, // Cache for imported generators
}

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    pub fn new(program: &'a CompiledProgram, rng: &'a mut R) -> Self {
        Evaluator {
            program,
            rng,
            variables: HashMap::new(),
            last_number: None,
            current_item: None,
            dynamic_properties: HashMap::new(),
            consumable_lists: HashMap::new(),
            consumable_list_counter: 0,
            loader: None,
            import_cache: HashMap::new(),
        }
    }

    /// Set the generator loader for handling imports
    pub fn with_loader(mut self, loader: Arc<dyn GeneratorLoader>) -> Self {
        self.loader = Some(loader);
        self
    }

    /// Load and compile an imported generator
    async fn load_import(&mut self, name: &str, span: Span) -> Result<&CompiledProgram, EvalError> {
        // Check cache first
        if self.import_cache.contains_key(name) {
            return Ok(self.import_cache.get(name).unwrap());
        }

        // Check if loader is available
        let loader = self.loader.as_ref().ok_or_else(|| EvalError::ImportError {
            message: "No loader available for imports".to_string(),
            span,
        })?;

        // Load the generator source
        let source = loader
            .load(name)
            .await
            .map_err(|e| EvalError::ImportError {
                message: format!("Failed to load generator '{}': {}", name, e),
                span,
            })?;

        // Parse and compile the generator
        let program = crate::parser::parse(&source).map_err(|e| EvalError::ImportError {
            message: format!("Failed to parse generator '{}': {}", name, e),
            span,
        })?;

        let compiled = crate::compiler::compile(&program).map_err(|e| EvalError::ImportError {
            message: format!("Failed to compile generator '{}': {}", name, e),
            span,
        })?;

        // Cache it
        self.import_cache.insert(name.to_string(), compiled);

        Ok(self.import_cache.get(name).unwrap())
    }

    pub async fn evaluate(&mut self) -> Result<String, EvalError> {
        // Priority order: $output, output, then last list
        // Check for $output list first (top-level $output = ...)
        if let Some(output_list) = self.program.get_list("$output") {
            return self.evaluate_list(output_list).await;
        }

        // Check for output list
        match self.program.get_list("output") {
            Some(output_list) => self.evaluate_list(output_list).await,
            None => {
                // Default to the last list if no "output" list is defined
                if let Some(last_list_name) = self.program.list_order.last() {
                    if let Some(last_list) = self.program.get_list(last_list_name) {
                        self.evaluate_list(last_list).await
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
    }

    #[async_recursion]
    async fn evaluate_list(&mut self, list: &CompiledList) -> Result<String, EvalError> {
        if list.items.is_empty() && list.output.is_none() {
            return Err(EvalError::EmptyList {
                name: list.name.clone(),
                span: Span::dummy(),
            });
        }

        // Select an item based on weights (if there are items)
        let item = if !list.items.is_empty() {
            Some(
                self.select_weighted_item(&list.items, list.total_weight)
                    .await?
                    .clone(),
            )
        } else {
            None
        };

        // Check if list has $output property
        if let Some(output_content) = &list.output {
            // Set current_item for `this` keyword access
            let old_item = self.current_item.take();
            let old_dynamic_properties = std::mem::take(&mut self.dynamic_properties);

            if let Some(ref selected_item) = item {
                self.current_item = Some(selected_item.clone());
            }

            let result = self.evaluate_content(output_content).await;

            // Restore previous context
            self.current_item = old_item;
            self.dynamic_properties = old_dynamic_properties;

            return result;
        }

        // No $output, use normal evaluation
        let item = item.unwrap(); // Safe because we checked items.is_empty() above

        // If the item has sublists, first select a sublist, then select from it
        if !item.sublists.is_empty() {
            // Randomly select a sublist
            let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
            let idx = self.rng.gen_range(0..sublist_names.len());
            let sublist_name = &sublist_names[idx];
            let sublist = item.sublists.get(sublist_name).unwrap();
            return self.evaluate_list(sublist).await;
        }

        // Evaluate the item's content
        self.evaluate_content(&item.content).await
    }

    #[async_recursion]
    async fn select_weighted_item<'b>(
        &mut self,
        items: &'b [CompiledItem],
        _total_weight: f64,
    ) -> Result<&'b CompiledItem, EvalError> {
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
                return Ok(&items[i]);
            }
        }

        // Fallback to last item (in case of floating point errors)
        Ok(&items[items.len() - 1])
    }

    #[async_recursion]
    async fn evaluate_content(
        &mut self,
        content: &[Spanned<ContentPart>],
    ) -> Result<String, EvalError> {
        let mut result = String::new();

        for (i, part_spanned) in content.iter().enumerate() {
            let part = &part_spanned.value;
            match part {
                ContentPart::Text(text) => {
                    result.push_str(text);
                    // Track numbers for {s} pluralization
                    if let Some(num) = self.extract_number(text) {
                        self.last_number = Some(num);
                    }
                }
                ContentPart::Escape(ch) => result.push(*ch),
                ContentPart::Reference(expr_spanned) => {
                    let value = self.evaluate_expression(expr_spanned).await?;
                    // Track numbers for {s} pluralization
                    if let Ok(num) = value.parse::<i64>() {
                        self.last_number = Some(num);
                    }
                    result.push_str(&value);
                }
                ContentPart::Inline(inline_spanned) => {
                    let inline = &inline_spanned.value;
                    // Check if this is a special inline: {a} or {s}
                    if inline.choices.len() == 1 && inline.choices[0].value.content.len() == 1 {
                        match &inline.choices[0].value.content[0].value {
                            ContentPart::Article => {
                                // {a} - choose "a" or "an" based on next word
                                let next_word = self.peek_next_word(content, i + 1).await?;
                                if self.starts_with_vowel_sound(&next_word) {
                                    result.push_str("an");
                                } else {
                                    result.push('a');
                                }
                                continue;
                            }
                            ContentPart::Pluralize => {
                                // {s} - add "s" if last number != 1
                                if let Some(num) = self.last_number {
                                    if num != 1 && num != -1 {
                                        result.push('s');
                                    }
                                } else {
                                    // Default to plural if no number context
                                    result.push('s');
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Regular inline evaluation
                    let value = self.evaluate_inline(inline_spanned).await?;
                    // Track numbers for {s} pluralization
                    if let Ok(num) = value.parse::<i64>() {
                        self.last_number = Some(num);
                    }
                    result.push_str(&value);
                }
                ContentPart::Article => {
                    // {a} - choose "a" or "an" based on next word
                    let next_word = self.peek_next_word(content, i + 1).await?;
                    if self.starts_with_vowel_sound(&next_word) {
                        result.push_str("an");
                    } else {
                        result.push('a');
                    }
                }
                ContentPart::Pluralize => {
                    // {s} - add "s" if last number != 1
                    if let Some(num) = self.last_number {
                        if num != 1 && num != -1 {
                            result.push('s');
                        }
                    } else {
                        // Default to plural if no number context
                        result.push('s');
                    }
                }
            }
        }

        Ok(result)
    }

    #[async_recursion]
    async fn evaluate_inline(
        &mut self,
        inline_spanned: &Spanned<InlineList>,
    ) -> Result<String, EvalError> {
        let inline = &inline_spanned.value;
        if inline.choices.is_empty() {
            return Ok(String::new());
        }

        // Check if this is a special case (number range, letter range)
        if inline.choices.len() == 1 {
            if let Some(content_part_spanned) = inline.choices[0].value.content.first() {
                if let ContentPart::Reference(expr_spanned) = &content_part_spanned.value {
                    match &expr_spanned.value {
                        Expression::NumberRange(start, end) => {
                            let num = self.rng.gen_range(*start..=*end);
                            return Ok(num.to_string());
                        }
                        Expression::LetterRange(start, end) => {
                            let start_byte = *start as u8;
                            let end_byte = *end as u8;
                            let random_byte = self.rng.gen_range(start_byte..=end_byte);
                            return Ok((random_byte as char).to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Calculate actual weights for choices with dynamic weights
        let mut actual_weights: Vec<f64> = Vec::new();
        let mut actual_total = 0.0;

        for choice_spanned in &inline.choices {
            let choice = &choice_spanned.value;
            let weight = match &choice.weight {
                Some(ItemWeight::Static(w)) => *w,
                Some(ItemWeight::Dynamic(expr_spanned)) => {
                    // Evaluate the dynamic weight expression
                    let result = self.evaluate_expression(expr_spanned).await?;
                    // Convert to number: "true" -> 1.0, "false" -> 0.0, or parse as number
                    let weight = if result == "true" {
                        1.0
                    } else if result == "false" || result.is_empty() {
                        0.0
                    } else {
                        result.parse::<f64>().unwrap_or(0.0)
                    };
                    weight.max(0.0)
                }
                None => 1.0,
            };
            actual_weights.push(weight);
            actual_total += weight;
        }

        if actual_total <= 0.0 {
            // If all weights are 0, treat all choices as having equal weight (1.0)
            actual_weights = vec![1.0; inline.choices.len()];
            actual_total = inline.choices.len() as f64;
        }

        // Select a choice
        let random_value = self.rng.gen::<f64>() * actual_total;
        let mut cumulative = 0.0;

        for (i, weight) in actual_weights.iter().enumerate() {
            cumulative += weight;
            if random_value < cumulative {
                return self
                    .evaluate_content(&inline.choices[i].value.content)
                    .await;
            }
        }

        // Fallback
        self.evaluate_content(&inline.choices[inline.choices.len() - 1].value.content)
            .await
    }

    // Helper function to extract a number from text
    fn extract_number(&self, text: &str) -> Option<i64> {
        // Look for any number in the text (last one wins)
        let mut last_num = None;
        for word in text.split_whitespace() {
            if let Ok(num) = word
                .trim_matches(|c: char| !c.is_ascii_digit() && c != '-')
                .parse::<i64>()
            {
                last_num = Some(num);
            }
        }
        last_num
    }

    // Helper function to peek at the next word in content
    #[async_recursion]
    async fn peek_next_word(
        &mut self,
        content: &[Spanned<ContentPart>],
        start_idx: usize,
    ) -> Result<String, EvalError> {
        for part_spanned in &content[start_idx..] {
            match &part_spanned.value {
                ContentPart::Text(text) => {
                    // Get the first word from the text
                    if let Some(word) = text.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                ContentPart::Reference(expr_spanned) => {
                    // Evaluate the reference to get the word
                    let value = self.evaluate_expression(expr_spanned).await?;
                    if let Some(word) = value.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                ContentPart::Inline(inline_spanned) => {
                    // Evaluate the inline to get the word
                    let value = self.evaluate_inline(inline_spanned).await?;
                    if let Some(word) = value.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        // If no word found, default to consonant article "a"
        Ok(String::from("word"))
    }

    // Helper function to check if a word starts with a vowel sound
    fn starts_with_vowel_sound(&self, word: &str) -> bool {
        if word.is_empty() {
            return false;
        }

        let first_char = word.chars().next().unwrap().to_ascii_lowercase();

        // Simple vowel check - could be enhanced with special cases
        // (e.g., "hour" starts with vowel sound, "university" doesn't)
        matches!(first_char, 'a' | 'e' | 'i' | 'o' | 'u')
    }

    #[async_recursion]
    async fn evaluate_expression(
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
                    Some(list) => self.evaluate_list(list).await,
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
                                return self.evaluate_list(&sublist_clone).await;
                            }

                            // If the item has exactly one sublist, delegate to it
                            if item.sublists.len() == 1 {
                                let single_sublist = item.sublists.values().next().unwrap();
                                // Search through items in the single sublist for the property
                                for subitem in &single_sublist.items {
                                    if let Some(target_sublist) = subitem.sublists.get(&prop.name) {
                                        let target_clone = target_sublist.clone();
                                        return self.evaluate_list(&target_clone).await;
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
                // Load the imported generator
                let imported_program = self.load_import(generator_name, span).await?.clone();

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

    fn is_truthy(&self, s: &str) -> bool {
        // Empty string, "false", "0" are falsy
        !s.is_empty() && s != "false" && s != "0"
    }

    fn compare_values(&self, left: &str, right: &str) -> Result<i32, EvalError> {
        // Try to parse as numbers first
        if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
            if l < r {
                Ok(-1)
            } else if l > r {
                Ok(1)
            } else {
                Ok(0)
            }
        } else {
            // String comparison
            Ok(left.cmp(right) as i32)
        }
    }

    fn format_number(&self, num: f64) -> String {
        // Format number without unnecessary decimal points
        if num.fract() == 0.0 && num.abs() < 1e15 {
            // It's an integer (or very close to one)
            format!("{}", num as i64)
        } else {
            // It's a float, format with precision
            format!("{}", num)
        }
    }

    #[async_recursion]
    async fn evaluate_to_value(
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

    #[async_recursion]
    async fn value_to_string(&mut self, value: Value) -> Result<String, EvalError> {
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
                self.evaluate_list(list).await
            }
            Value::ListInstance(list) => self.evaluate_list(&list).await,
            Value::ItemInstance(item) => {
                // Evaluate the item's content
                // If it has sublists, pick one randomly
                if !item.sublists.is_empty() {
                    let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                    let idx = self.rng.gen_range(0..sublist_names.len());
                    let sublist_name = &sublist_names[idx];
                    let sublist = item.sublists.get(sublist_name).unwrap();
                    self.evaluate_list(sublist).await
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
                    self.evaluate_list(sublist).await
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

    /// Evaluate a list to a Value (useful for checking if it evaluates to an ImportedGenerator)
    async fn evaluate_list_to_value(&mut self, list: &CompiledList) -> Result<Value, EvalError> {
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
        let result = self.evaluate_list(list).await?;
        Ok(Value::Text(result))
    }

    #[async_recursion]
    async fn get_property_value(
        &mut self,
        value: &Value,
        prop_name: &str,
        span: Span,
    ) -> Result<Value, EvalError> {
        match value {
            Value::List(list_name) => {
                // Look up the list
                let list =
                    self.program
                        .get_list(list_name)
                        .ok_or_else(|| EvalError::UndefinedList {
                            name: list_name.clone(),
                            span,
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

                Err(EvalError::UndefinedProperty {
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

                Err(EvalError::UndefinedProperty {
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
                    Err(EvalError::UndefinedProperty {
                        list: "item".to_string(),
                        prop: prop_name.to_string(),
                        span,
                    })
                } else {
                    Err(EvalError::UndefinedProperty {
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
                Err(EvalError::TypeError {
                    message: format!("Cannot access property '{}' on text value", prop_name),
                    span,
                })
            }
            Value::Array(_) => Err(EvalError::TypeError {
                message: format!("Cannot access property '{}' on array value", prop_name),
                span,
            }),
            Value::ConsumableList(_) => {
                // Check if this is a method that can be applied to consumable lists
                if self.is_grammar_method(prop_name) || prop_name == "selectOne" {
                    let method = MethodCall::new(prop_name.to_string());
                    return self.call_method_value(value, &method, span).await;
                }
                Err(EvalError::TypeError {
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
                    Err(EvalError::UndefinedProperty {
                        list: generator_name.clone(),
                        prop: prop_name.to_string(),
                        span,
                    })
                }
            }
        }
    }

    fn is_grammar_method(&self, name: &str) -> bool {
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
    fn extract_simple_list_reference(content: &[Spanned<ContentPart>]) -> Option<String> {
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
    async fn get_property(
        &mut self,
        value: &Value,
        prop_name: &str,
        span: Span,
    ) -> Result<String, EvalError> {
        let prop_value = self.get_property_value(value, prop_name, span).await?;
        self.value_to_string(prop_value).await
    }

    #[async_recursion]
    async fn call_method(
        &mut self,
        value: &Value,
        method: &MethodCall,
        span: Span,
    ) -> Result<String, EvalError> {
        let value_result = self.call_method_value(value, method, span).await?;
        self.value_to_string(value_result).await
    }

    #[async_recursion]
    async fn call_method_value(
        &mut self,
        value: &Value,
        method: &MethodCall,
        span: Span,
    ) -> Result<Value, EvalError> {
        match method.name.as_str() {
            "selectOne" => {
                // Select one item from the list and return it as a Value
                match value {
                    Value::List(name) => {
                        let list = self.program.get_list(name).ok_or_else(|| {
                            EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            }
                        })?;

                        let item = self
                            .select_weighted_item(&list.items, list.total_weight)
                            .await?
                            .clone();

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
                        let item = self
                            .select_weighted_item(&list.items, list.total_weight)
                            .await?
                            .clone();

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
                            EvalError::UndefinedList {
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
                                        results.push(self.evaluate_list(sublist).await?);
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
                                        results.push(self.evaluate_list(sublist).await?);
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
                        Err(EvalError::InvalidMethodCall {
                            message: "selectAll cannot be called on consumable lists".to_string(),
                            span,
                        })
                    }
                    Value::ImportedGenerator(_) => {
                        // selectAll is not meaningful for imported generators
                        Err(EvalError::InvalidMethodCall {
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
                    return Err(EvalError::InvalidMethodCall {
                        message: "selectMany requires at least one argument".to_string(),
                        span,
                    });
                } else if method.args.len() == 1 {
                    // Single argument: exact count
                    let arg_str = self.evaluate_expression(&method.args[0]).await?;
                    arg_str
                        .parse::<usize>()
                        .map_err(|_| EvalError::InvalidMethodCall {
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
                    let min =
                        min_str
                            .parse::<usize>()
                            .map_err(|_| EvalError::InvalidMethodCall {
                                message: format!(
                                    "selectMany min argument must be a number, got: {}",
                                    min_str
                                ),
                                span,
                            })?;
                    let max =
                        max_str
                            .parse::<usize>()
                            .map_err(|_| EvalError::InvalidMethodCall {
                                message: format!(
                                    "selectMany max argument must be a number, got: {}",
                                    max_str
                                ),
                                span,
                            })?;
                    if min > max {
                        return Err(EvalError::InvalidMethodCall {
                            message: format!(
                                "selectMany min ({}) cannot be greater than max ({})",
                                min, max
                            ),
                            span,
                        });
                    }
                    self.rng.gen_range(min..=max)
                } else {
                    return Err(EvalError::InvalidMethodCall {
                        message: "selectMany accepts 1 or 2 arguments".to_string(),
                        span,
                    });
                };

                match value {
                    Value::List(name) => {
                        let list = self.program.get_list(name).ok_or_else(|| {
                            EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            }
                        })?;

                        let mut results = Vec::new();
                        for _ in 0..n {
                            let item = self
                                .select_weighted_item(&list.items, list.total_weight)
                                .await?
                                .clone();
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let idx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[idx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist).await?);
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
                            let item = self
                                .select_weighted_item(&list.items, list.total_weight)
                                .await?
                                .clone();
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let idx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[idx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist).await?);
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
                        Err(EvalError::InvalidMethodCall { message: "selectMany cannot be called on consumable lists (use selectUnique instead)".to_string(), span })
                    }
                    Value::ImportedGenerator(_) => {
                        // selectMany is not meaningful for imported generators
                        Err(EvalError::InvalidMethodCall {
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
                    return Err(EvalError::InvalidMethodCall {
                        message: "selectUnique requires at least one argument".to_string(),
                        span,
                    });
                } else if method.args.len() == 1 {
                    // Single argument: exact count
                    let arg_str = self.evaluate_expression(&method.args[0]).await?;
                    arg_str
                        .parse::<usize>()
                        .map_err(|_| EvalError::InvalidMethodCall {
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
                    let min =
                        min_str
                            .parse::<usize>()
                            .map_err(|_| EvalError::InvalidMethodCall {
                                message: format!(
                                    "selectUnique min argument must be a number, got: {}",
                                    min_str
                                ),
                                span,
                            })?;
                    let max =
                        max_str
                            .parse::<usize>()
                            .map_err(|_| EvalError::InvalidMethodCall {
                                message: format!(
                                    "selectUnique max argument must be a number, got: {}",
                                    max_str
                                ),
                                span,
                            })?;
                    if min > max {
                        return Err(EvalError::InvalidMethodCall {
                            message: format!(
                                "selectUnique min ({}) cannot be greater than max ({})",
                                min, max
                            ),
                            span,
                        });
                    }
                    self.rng.gen_range(min..=max)
                } else {
                    return Err(EvalError::InvalidMethodCall {
                        message: "selectUnique accepts 1 or 2 arguments".to_string(),
                        span,
                    });
                };

                match value {
                    Value::List(name) => {
                        let list = self.program.get_list(name).ok_or_else(|| {
                            EvalError::UndefinedList {
                                name: name.clone(),
                                span,
                            }
                        })?;

                        if n > list.items.len() {
                            return Err(EvalError::InvalidMethodCall {
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
                                    results.push(self.evaluate_list(sublist).await?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content).await?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ListInstance(list) => {
                        if n > list.items.len() {
                            return Err(EvalError::InvalidMethodCall {
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
                                    results.push(self.evaluate_list(sublist).await?);
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
                            return Err(EvalError::InvalidMethodCall {
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
                            return Err(EvalError::InvalidMethodCall {
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
                        Err(EvalError::InvalidMethodCall {
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
                            .ok_or_else(|| EvalError::UndefinedList {
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
                                .ok_or_else(|| EvalError::ImportError {
                                    message: format!(
                                        "Cannot find output list in imported generator '{}'",
                                        generator_name
                                    ),
                                    span,
                                })?
                                .clone()
                        } else {
                            return Err(EvalError::ImportError {
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
                    _ => Err(EvalError::InvalidMethodCall {
                        message: "consumableList can only be called on lists".to_string(),
                        span,
                    }),
                }
            }

            "joinLists" => {
                // Join multiple lists into a single list
                // This is a built-in function that mimics the join-lists-plugin

                if method.args.is_empty() {
                    return Err(EvalError::InvalidMethodCall {
                        message: "joinLists requires at least one argument".to_string(),
                        span,
                    });
                }

                // Collect all items from all list arguments
                let mut combined_items = Vec::new();

                for arg in &method.args {
                    // Evaluate the argument to get a list value
                    let list_value = self.evaluate_to_value(arg).await?;

                    // Get the items from the list
                    match list_value {
                        Value::List(name) => {
                            let list = self.program.get_list(&name).ok_or_else(|| {
                                EvalError::UndefinedList {
                                    name: name.clone(),
                                    span,
                                }
                            })?;
                            combined_items.extend(list.items.clone());
                        }
                        Value::ListInstance(list) => {
                            combined_items.extend(list.items.clone());
                        }
                        Value::Text(_)
                        | Value::Array(_)
                        | Value::ItemInstance(_)
                        | Value::ConsumableList(_)
                        | Value::ImportedGenerator(_) => {
                            return Err(EvalError::TypeError {
                                message: format!(
                                    "joinLists arguments must be lists, got {:?}",
                                    list_value
                                ),
                                span,
                            });
                        }
                    }
                }

                // Create a new list with all combined items
                let combined_list = CompiledList {
                    name: "__joined__".to_string(),
                    items: combined_items.clone(),
                    total_weight: combined_items.iter().map(|item| item.weight).sum(),
                    output: None,
                };

                Ok(Value::ListInstance(combined_list))
            }

            _ => Err(EvalError::InvalidMethodCall {
                message: format!("Unknown method: {}", method.name),
                span,
            }),
        }
    }
}

fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn to_sentence_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn to_plural(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular plurals
    let irregulars = [
        ("child", "children"),
        ("person", "people"),
        ("man", "men"),
        ("woman", "women"),
        ("tooth", "teeth"),
        ("foot", "feet"),
        ("mouse", "mice"),
        ("goose", "geese"),
        ("ox", "oxen"),
        ("sheep", "sheep"),
        ("deer", "deer"),
        ("fish", "fish"),
    ];

    for (singular, plural) in &irregulars {
        if lower == *singular {
            return plural.to_string();
        }
    }

    // Regular plural rules
    if lower.ends_with("s")
        || lower.ends_with("ss")
        || lower.ends_with("sh")
        || lower.ends_with("ch")
        || lower.ends_with("x")
        || lower.ends_with("z")
    {
        return format!("{}es", s_trimmed);
    } else if lower.ends_with("y") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}ies", &s_trimmed[..s_trimmed.len() - 1]);
        }
    } else if lower.ends_with("f") {
        return format!("{}ves", &s_trimmed[..s_trimmed.len() - 1]);
    } else if lower.ends_with("fe") {
        return format!("{}ves", &s_trimmed[..s_trimmed.len() - 2]);
    } else if lower.ends_with("o") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}es", s_trimmed);
        }
    }

    // Default: add 's'
    format!("{}s", s_trimmed)
}

fn to_past_tense(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular verbs
    let irregulars = [
        ("be", "was"),
        ("have", "had"),
        ("do", "did"),
        ("say", "said"),
        ("go", "went"),
        ("get", "got"),
        ("make", "made"),
        ("know", "knew"),
        ("think", "thought"),
        ("take", "took"),
        ("see", "saw"),
        ("come", "came"),
        ("want", "wanted"),
        ("give", "gave"),
        ("use", "used"),
        ("find", "found"),
        ("tell", "told"),
        ("ask", "asked"),
        ("work", "worked"),
        ("feel", "felt"),
        ("leave", "left"),
        ("put", "put"),
        ("mean", "meant"),
        ("keep", "kept"),
        ("let", "let"),
        ("begin", "began"),
        ("seem", "seemed"),
        ("help", "helped"),
        ("show", "showed"),
        ("hear", "heard"),
        ("play", "played"),
        ("run", "ran"),
        ("move", "moved"),
        ("live", "lived"),
        ("believe", "believed"),
        ("bring", "brought"),
        ("write", "wrote"),
        ("sit", "sat"),
        ("stand", "stood"),
        ("lose", "lost"),
        ("pay", "paid"),
        ("meet", "met"),
        ("include", "included"),
        ("continue", "continued"),
        ("set", "set"),
        ("learn", "learned"),
        ("change", "changed"),
        ("lead", "led"),
        ("understand", "understood"),
        ("watch", "watched"),
        ("follow", "followed"),
        ("stop", "stopped"),
        ("create", "created"),
        ("speak", "spoke"),
        ("read", "read"),
        ("spend", "spent"),
        ("grow", "grew"),
        ("open", "opened"),
        ("walk", "walked"),
        ("win", "won"),
        ("teach", "taught"),
        ("offer", "offered"),
        ("remember", "remembered"),
        ("consider", "considered"),
        ("appear", "appeared"),
        ("buy", "bought"),
        ("serve", "served"),
        ("die", "died"),
        ("send", "sent"),
        ("build", "built"),
        ("stay", "stayed"),
        ("fall", "fell"),
        ("cut", "cut"),
        ("reach", "reached"),
        ("kill", "killed"),
        ("raise", "raised"),
        ("pass", "passed"),
        ("sell", "sold"),
        ("decide", "decided"),
        ("return", "returned"),
        ("explain", "explained"),
        ("hope", "hoped"),
        ("develop", "developed"),
        ("carry", "carried"),
        ("break", "broke"),
        ("receive", "received"),
        ("agree", "agreed"),
        ("support", "supported"),
        ("hit", "hit"),
        ("produce", "produced"),
        ("eat", "ate"),
        ("cover", "covered"),
        ("catch", "caught"),
        ("draw", "drew"),
    ];

    for (present, past) in &irregulars {
        if lower == *present {
            return past.to_string();
        }
    }

    // Regular past tense rules
    if lower.ends_with("e") {
        return format!("{}d", s_trimmed);
    } else if lower.ends_with("y") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}ied", &s_trimmed[..s_trimmed.len() - 1]);
        }
    }

    // Default: add 'ed'
    format!("{}ed", s_trimmed)
}

fn to_possessive(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    // If it ends with 's', just add apostrophe
    // Otherwise add apostrophe + s
    if s_trimmed.ends_with('s') {
        format!("{}'", s_trimmed)
    } else {
        format!("{}'s", s_trimmed)
    }
}

fn to_future_tense(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    // Future tense in English is typically "will" + base form
    format!("will {}", s_trimmed)
}

fn to_present_tense(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular present tense (third person singular)
    let irregulars = [
        ("be", "is"),
        ("have", "has"),
        ("do", "does"),
        ("go", "goes"),
        ("was", "is"),
        ("were", "are"),
        ("had", "has"),
        ("did", "does"),
        ("went", "goes"),
        ("got", "gets"),
        ("made", "makes"),
        ("knew", "knows"),
        ("thought", "thinks"),
        ("took", "takes"),
        ("saw", "sees"),
        ("came", "comes"),
        ("gave", "gives"),
        ("found", "finds"),
        ("told", "tells"),
        ("asked", "asks"),
        ("felt", "feels"),
        ("left", "leaves"),
        ("put", "puts"),
        ("meant", "means"),
        ("kept", "keeps"),
        ("let", "lets"),
        ("began", "begins"),
        ("seemed", "seems"),
        ("showed", "shows"),
        ("heard", "hears"),
        ("ran", "runs"),
        ("moved", "moves"),
        ("lived", "lives"),
        ("brought", "brings"),
        ("wrote", "writes"),
        ("sat", "sits"),
        ("stood", "stands"),
        ("lost", "loses"),
        ("paid", "pays"),
        ("met", "meets"),
        ("set", "sets"),
        ("led", "leads"),
        ("understood", "understands"),
        ("followed", "follows"),
        ("stopped", "stops"),
        ("spoke", "speaks"),
        ("read", "reads"),
        ("spent", "spends"),
        ("grew", "grows"),
        ("walked", "walks"),
        ("won", "wins"),
        ("taught", "teaches"),
        ("remembered", "remembers"),
        ("appeared", "appears"),
        ("bought", "buys"),
        ("served", "serves"),
        ("died", "dies"),
        ("sent", "sends"),
        ("built", "builds"),
        ("stayed", "stays"),
        ("fell", "falls"),
        ("cut", "cuts"),
        ("reached", "reaches"),
        ("killed", "kills"),
        ("raised", "raises"),
        ("passed", "passes"),
        ("sold", "sells"),
        ("decided", "decides"),
        ("returned", "returns"),
        ("explained", "explains"),
        ("hoped", "hopes"),
        ("carried", "carries"),
        ("broke", "breaks"),
        ("received", "receives"),
        ("agreed", "agrees"),
        ("hit", "hits"),
        ("produced", "produces"),
        ("ate", "eats"),
        ("caught", "catches"),
        ("drew", "draws"),
    ];

    for (past, present) in &irregulars {
        if lower == *past {
            return present.to_string();
        }
    }

    // If it already looks like present tense (ends with common patterns)
    if lower.ends_with("s") || lower.ends_with("es") {
        return s_trimmed.to_string();
    }

    // Regular present tense (third person singular)
    if lower.ends_with("y") && s_trimmed.len() > 1 {
        let second_last = s_trimmed.chars().nth(s_trimmed.len() - 2).unwrap();
        if !"aeiou".contains(second_last.to_ascii_lowercase()) {
            return format!("{}ies", &s_trimmed[..s_trimmed.len() - 1]);
        }
    } else if lower.ends_with("s")
        || lower.ends_with("ss")
        || lower.ends_with("sh")
        || lower.ends_with("ch")
        || lower.ends_with("x")
        || lower.ends_with("z")
        || lower.ends_with("o")
    {
        return format!("{}es", s_trimmed);
    }

    // Default: add 's'
    format!("{}s", s_trimmed)
}

fn to_negative_form(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Special cases for common verbs - all add "not" after the verb
    if lower == "is"
        || lower == "are"
        || lower == "am"
        || lower == "was"
        || lower == "were"
        || lower == "have"
        || lower == "has"
        || lower == "had"
        || lower == "do"
        || lower == "does"
        || lower == "did"
        || lower == "will"
        || lower == "would"
        || lower == "should"
        || lower == "could"
        || lower == "can"
        || lower == "may"
        || lower == "might"
        || lower == "must"
    {
        return format!("{} not", s_trimmed);
    }

    // For regular verbs, use "does not" + base form
    // This is a simplification; ideally we'd convert to base form
    format!("does not {}", s_trimmed)
}

fn to_singular(s: &str) -> String {
    let s_trimmed = s.trim();
    if s_trimmed.is_empty() {
        return s.to_string();
    }

    let lower = s_trimmed.to_lowercase();

    // Common irregular plurals (reversed from to_plural)
    let irregulars = [
        ("children", "child"),
        ("people", "person"),
        ("men", "man"),
        ("women", "woman"),
        ("teeth", "tooth"),
        ("feet", "foot"),
        ("mice", "mouse"),
        ("geese", "goose"),
        ("oxen", "ox"),
        ("sheep", "sheep"),
        ("deer", "deer"),
        ("fish", "fish"),
    ];

    for (plural, singular) in &irregulars {
        if lower == *plural {
            return singular.to_string();
        }
    }

    // Regular plural rules (reversed)
    if lower.ends_with("ies") && s_trimmed.len() > 3 {
        return format!("{}y", &s_trimmed[..s_trimmed.len() - 3]);
    } else if lower.ends_with("ves") && s_trimmed.len() > 3 {
        // Could be knife -> knives or life -> lives
        return format!("{}fe", &s_trimmed[..s_trimmed.len() - 3]);
    } else if lower.ends_with("oes") && s_trimmed.len() > 3 {
        return format!("{}o", &s_trimmed[..s_trimmed.len() - 2]);
    } else if lower.ends_with("ses") && s_trimmed.len() > 3 {
        return s_trimmed[..s_trimmed.len() - 2].to_string();
    } else if lower.ends_with("xes")
        || lower.ends_with("zes")
        || lower.ends_with("ches")
        || lower.ends_with("shes")
    {
        if s_trimmed.len() > 2 {
            return s_trimmed[..s_trimmed.len() - 2].to_string();
        }
    } else if lower.ends_with("s") && !lower.ends_with("ss") {
        // Simple plural - just remove 's'
        if s_trimmed.len() > 1 {
            return s_trimmed[..s_trimmed.len() - 1].to_string();
        }
    }

    // If no rule matched, return as-is
    s_trimmed.to_string()
}

pub async fn evaluate<R: Rng + Send>(
    program: &CompiledProgram,
    rng: &mut R,
) -> Result<String, EvalError> {
    let mut evaluator = Evaluator::new(program, rng);
    evaluator.evaluate().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::parser::parse;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[tokio::test]
    async fn test_simple_evaluation() {
        let input = "output\n\thello world\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        assert_eq!(result.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn test_list_reference() {
        let input = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a [animal].\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        let output = result.unwrap();
        assert!(output == "I saw a dog." || output == "I saw a cat.");
    }

    #[tokio::test]
    async fn test_deterministic() {
        let input = "animal\n\tdog\n\tcat\n\noutput\n\t[animal]\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();

        let mut rng1 = StdRng::seed_from_u64(12345);
        let result1 = evaluate(&compiled, &mut rng1).await.unwrap();

        let mut rng2 = StdRng::seed_from_u64(12345);
        let result2 = evaluate(&compiled, &mut rng2).await.unwrap();

        assert_eq!(result1, result2);
    }

    #[tokio::test]
    async fn test_inline_list() {
        let input = "output\n\t{big|small}\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        let output = result.unwrap();
        assert!(output == "big" || output == "small");
    }

    #[tokio::test]
    async fn test_number_range() {
        let input = "output\n\t{1-6}\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        let output = result.unwrap();
        let num: i32 = output.parse().unwrap();
        assert!((1..=6).contains(&num));
    }
}
