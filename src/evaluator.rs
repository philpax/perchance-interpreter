/// Evaluator executes compiled programs with RNG support
use crate::ast::*;
use crate::compiler::*;
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    UndefinedList(String),
    UndefinedVariable(String),
    UndefinedProperty(String, String),
    InvalidMethodCall(String),
    EmptyList(String),
    TypeError(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UndefinedList(name) => write!(f, "Undefined list: {}", name),
            EvalError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            EvalError::UndefinedProperty(list, prop) => {
                write!(f, "Undefined property '{}' on list '{}'", prop, list)
            }
            EvalError::InvalidMethodCall(msg) => write!(f, "Invalid method call: {}", msg),
            EvalError::EmptyList(name) => write!(f, "Cannot select from empty list: {}", name),
            EvalError::TypeError(msg) => write!(f, "Type error: {}", msg),
        }
    }
}

impl std::error::Error for EvalError {}

#[derive(Debug, Clone)]
struct ConsumableListState {
    source_list_name: String,
    remaining_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
enum Value {
    Text(String),
    List(String),               // Reference to a list by name
    ListInstance(CompiledList), // An actual list instance (for sublists)
    Array(Vec<String>),         // Multiple items (for selectMany/selectUnique before joinItems)
    ConsumableList(String),     // Reference to a consumable list by unique ID
}

pub struct Evaluator<'a, R: Rng> {
    program: &'a CompiledProgram,
    rng: &'a mut R,
    variables: HashMap<String, Value>,
    last_number: Option<i64>, // Track last number for {s} pluralization
    current_item: Option<CompiledItem>, // Track current item for `this` keyword
    consumable_lists: HashMap<String, ConsumableListState>, // Track consumable lists
    consumable_list_counter: usize, // Counter for generating unique IDs
}

impl<'a, R: Rng> Evaluator<'a, R> {
    pub fn new(program: &'a CompiledProgram, rng: &'a mut R) -> Self {
        Evaluator {
            program,
            rng,
            variables: HashMap::new(),
            last_number: None,
            current_item: None,
            consumable_lists: HashMap::new(),
            consumable_list_counter: 0,
        }
    }

    pub fn evaluate(&mut self) -> Result<String, EvalError> {
        // Evaluate the "output" list
        match self.program.get_list("output") {
            Some(output_list) => self.evaluate_list(output_list),
            None => Err(EvalError::UndefinedList("output".to_string())),
        }
    }

    fn evaluate_list(&mut self, list: &CompiledList) -> Result<String, EvalError> {
        if list.items.is_empty() && list.output.is_none() {
            return Err(EvalError::EmptyList(list.name.clone()));
        }

        // Select an item based on weights (if there are items)
        let item = if !list.items.is_empty() {
            Some(
                self.select_weighted_item(&list.items, list.total_weight)?
                    .clone(),
            )
        } else {
            None
        };

        // Check if list has $output property
        if let Some(output_content) = &list.output {
            // Set current_item for `this` keyword access
            let old_item = self.current_item.take();
            if let Some(ref selected_item) = item {
                self.current_item = Some(selected_item.clone());
            }

            let result = self.evaluate_content(output_content);

            // Restore previous context
            self.current_item = old_item;

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
            return self.evaluate_list(sublist);
        }

        // Evaluate the item's content
        self.evaluate_content(&item.content)
    }

    fn select_weighted_item<'b>(
        &mut self,
        items: &'b [CompiledItem],
        total_weight: f64,
    ) -> Result<&'b CompiledItem, EvalError> {
        if items.is_empty() {
            return Err(EvalError::EmptyList("(anonymous)".to_string()));
        }

        let random_value = self.rng.gen::<f64>() * total_weight;
        let mut cumulative = 0.0;

        for item in items {
            cumulative += item.weight;
            if random_value < cumulative {
                return Ok(item);
            }
        }

        // Fallback to last item (in case of floating point errors)
        Ok(&items[items.len() - 1])
    }

    fn evaluate_content(&mut self, content: &[ContentPart]) -> Result<String, EvalError> {
        let mut result = String::new();

        for (i, part) in content.iter().enumerate() {
            match part {
                ContentPart::Text(text) => {
                    result.push_str(text);
                    // Track numbers for {s} pluralization
                    if let Some(num) = self.extract_number(text) {
                        self.last_number = Some(num);
                    }
                }
                ContentPart::Escape(ch) => result.push(*ch),
                ContentPart::Reference(expr) => {
                    let value = self.evaluate_expression(expr)?;
                    // Track numbers for {s} pluralization
                    if let Ok(num) = value.parse::<i64>() {
                        self.last_number = Some(num);
                    }
                    result.push_str(&value);
                }
                ContentPart::Inline(inline) => {
                    // Check if this is a special inline: {a} or {s}
                    if inline.choices.len() == 1 && inline.choices[0].content.len() == 1 {
                        match &inline.choices[0].content[0] {
                            ContentPart::Article => {
                                // {a} - choose "a" or "an" based on next word
                                let next_word = self.peek_next_word(content, i + 1)?;
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
                    let value = self.evaluate_inline(inline)?;
                    // Track numbers for {s} pluralization
                    if let Ok(num) = value.parse::<i64>() {
                        self.last_number = Some(num);
                    }
                    result.push_str(&value);
                }
                ContentPart::Article => {
                    // {a} - choose "a" or "an" based on next word
                    let next_word = self.peek_next_word(content, i + 1)?;
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

    fn evaluate_inline(&mut self, inline: &InlineList) -> Result<String, EvalError> {
        if inline.choices.is_empty() {
            return Ok(String::new());
        }

        // Check if this is a special case (number range, letter range)
        if inline.choices.len() == 1 {
            if let Some(ContentPart::Reference(expr)) = inline.choices[0].content.first() {
                match expr {
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

        // Calculate total weight
        let total_weight: f64 = inline.choices.iter().map(|c| c.weight.unwrap_or(1.0)).sum();

        // Select a choice
        let random_value = self.rng.gen::<f64>() * total_weight;
        let mut cumulative = 0.0;

        for choice in &inline.choices {
            cumulative += choice.weight.unwrap_or(1.0);
            if random_value < cumulative {
                return self.evaluate_content(&choice.content);
            }
        }

        // Fallback
        self.evaluate_content(&inline.choices[inline.choices.len() - 1].content)
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
    fn peek_next_word(
        &mut self,
        content: &[ContentPart],
        start_idx: usize,
    ) -> Result<String, EvalError> {
        for part in &content[start_idx..] {
            match part {
                ContentPart::Text(text) => {
                    // Get the first word from the text
                    if let Some(word) = text.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                ContentPart::Reference(expr) => {
                    // Evaluate the reference to get the word
                    let value = self.evaluate_expression(expr)?;
                    if let Some(word) = value.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                ContentPart::Inline(inline) => {
                    // Evaluate the inline to get the word
                    let value = self.evaluate_inline(inline)?;
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

    fn evaluate_expression(&mut self, expr: &Expression) -> Result<String, EvalError> {
        match expr {
            Expression::Simple(ident) => {
                // Check for "this" keyword
                if ident.name == "this" {
                    return Err(EvalError::TypeError(
                        "Cannot use 'this' without property access (use this.property)".to_string(),
                    ));
                }

                // Check if it's a variable first
                if let Some(value) = self.variables.get(&ident.name) {
                    return self.value_to_string(value.clone());
                }

                // Otherwise, look up the list and evaluate it
                match self.program.get_list(&ident.name) {
                    Some(list) => self.evaluate_list(list),
                    None => Err(EvalError::UndefinedList(ident.name.clone())),
                }
            }

            Expression::Property(base, prop) => {
                // Special handling for "this" keyword
                if let Expression::Simple(ident) = base.as_ref() {
                    if ident.name == "this" {
                        // Access property from current_item and evaluate it
                        if let Some(ref item) = self.current_item {
                            if let Some(sublist) = item.sublists.get(&prop.name) {
                                let sublist_clone = sublist.clone();
                                return self.evaluate_list(&sublist_clone);
                            } else {
                                return Err(EvalError::UndefinedProperty(
                                    "this".to_string(),
                                    prop.name.clone(),
                                ));
                            }
                        } else {
                            return Err(EvalError::TypeError(
                                "'this' keyword can only be used within $output".to_string(),
                            ));
                        }
                    }
                }

                let base_value = self.evaluate_to_value(base)?;
                // Try as property first, then as a zero-argument method
                match self.get_property(&base_value, &prop.name) {
                    Ok(result) => Ok(result),
                    Err(EvalError::UndefinedProperty(_, _)) => {
                        // Try as a method call with no arguments
                        let method = MethodCall::new(prop.name.clone());
                        self.call_method(&base_value, &method)
                    }
                    Err(e) => Err(e),
                }
            }

            Expression::Dynamic(base, index) => {
                let base_value = self.evaluate_to_value(base)?;
                let index_str = self.evaluate_expression(index)?;
                self.get_property(&base_value, &index_str)
            }

            Expression::Method(base, method) => {
                let base_value = self.evaluate_to_value(base)?;
                self.call_method(&base_value, method)
            }

            Expression::Assignment(ident, value) => {
                let val = self.evaluate_to_value(value)?;
                self.variables.insert(ident.name.clone(), val);
                Ok(String::new())
            }

            Expression::Sequence(exprs, output) => {
                // Evaluate all expressions in sequence
                for expr in exprs {
                    self.evaluate_expression(expr)?;
                }

                // Return the output expression if present
                if let Some(out_expr) = output {
                    self.evaluate_expression(out_expr)
                } else {
                    Ok(String::new())
                }
            }

            Expression::Literal(s) => {
                // Evaluate the literal string (it may contain references)
                // We need to parse and evaluate the string content
                // For now, we'll use a simple approach: re-parse the string as content
                match crate::parser::Parser::new(s).parse_content_until_newline() {
                    Ok(content) => self.evaluate_content(&content),
                    Err(_) => Ok(s.clone()), // Fallback to literal if parsing fails
                }
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

            Expression::Conditional(cond, true_expr, false_expr) => {
                // Evaluate condition
                let cond_result = self.evaluate_expression(cond)?;

                // Check if condition is truthy
                if self.is_truthy(&cond_result) {
                    self.evaluate_expression(true_expr)
                } else {
                    self.evaluate_expression(false_expr)
                }
            }

            Expression::BinaryOp(left, op, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;

                let result = match op {
                    BinaryOperator::Equal => left_val == right_val,
                    BinaryOperator::NotEqual => left_val != right_val,
                    BinaryOperator::LessThan => self.compare_values(&left_val, &right_val)? < 0,
                    BinaryOperator::GreaterThan => self.compare_values(&left_val, &right_val)? > 0,
                    BinaryOperator::LessEqual => self.compare_values(&left_val, &right_val)? <= 0,
                    BinaryOperator::GreaterEqual => {
                        self.compare_values(&left_val, &right_val)? >= 0
                    }
                    BinaryOperator::And => self.is_truthy(&left_val) && self.is_truthy(&right_val),
                    BinaryOperator::Or => self.is_truthy(&left_val) || self.is_truthy(&right_val),
                };

                Ok(if result { "true" } else { "false" }.to_string())
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

    fn evaluate_to_value(&mut self, expr: &Expression) -> Result<Value, EvalError> {
        match expr {
            Expression::Simple(ident) => {
                // Handle "this" keyword
                if ident.name == "this" {
                    return Err(EvalError::TypeError(
                        "Cannot use 'this' without property access (use this.property)".to_string(),
                    ));
                }

                // Check variables first
                if let Some(value) = self.variables.get(&ident.name) {
                    return Ok(value.clone());
                }

                // Check if it's a list reference
                if self.program.get_list(&ident.name).is_some() {
                    return Ok(Value::List(ident.name.clone()));
                }

                Err(EvalError::UndefinedList(ident.name.clone()))
            }

            Expression::Property(base, prop) => {
                // Special handling for "this" keyword
                if let Expression::Simple(ident) = base.as_ref() {
                    if ident.name == "this" {
                        // Access property from current_item
                        if let Some(ref item) = self.current_item {
                            if let Some(sublist) = item.sublists.get(&prop.name) {
                                return Ok(Value::ListInstance(sublist.clone()));
                            } else {
                                return Err(EvalError::UndefinedProperty(
                                    "this".to_string(),
                                    prop.name.clone(),
                                ));
                            }
                        } else {
                            return Err(EvalError::TypeError(
                                "'this' keyword can only be used within $output".to_string(),
                            ));
                        }
                    }
                }

                let base_value = self.evaluate_to_value(base)?;
                // Try as property first, then as a zero-argument method
                match self.get_property_value(&base_value, &prop.name) {
                    Ok(value) => Ok(value),
                    Err(EvalError::UndefinedProperty(_, _)) => {
                        // Try as a method call with no arguments
                        let method = MethodCall::new(prop.name.clone());
                        self.call_method_value(&base_value, &method)
                    }
                    Err(e) => Err(e),
                }
            }

            Expression::Method(base, method) => {
                let base_value = self.evaluate_to_value(base)?;
                self.call_method_value(&base_value, method)
            }

            _ => {
                let result = self.evaluate_expression(expr)?;
                Ok(Value::Text(result))
            }
        }
    }

    fn value_to_string(&mut self, value: Value) -> Result<String, EvalError> {
        match value {
            Value::Text(s) => Ok(s),
            Value::List(name) => {
                let list = self
                    .program
                    .get_list(&name)
                    .ok_or_else(|| EvalError::UndefinedList(name.clone()))?;
                self.evaluate_list(list)
            }
            Value::ListInstance(list) => self.evaluate_list(&list),
            Value::Array(items) => Ok(items.join(" ")), // Default: space-separated
            Value::ConsumableList(id) => {
                // Get the consumable list state
                let state = self.consumable_lists.get(&id).ok_or_else(|| {
                    EvalError::UndefinedList(format!("Consumable list not found: {}", id))
                })?;

                // Check if there are any items left
                if state.remaining_indices.is_empty() {
                    return Err(EvalError::EmptyList(format!(
                        "Consumable list '{}' has been exhausted",
                        state.source_list_name
                    )));
                }

                // Get the source list
                let source_list_name = state.source_list_name.clone();
                let source_list = self
                    .program
                    .get_list(&source_list_name)
                    .ok_or_else(|| EvalError::UndefinedList(source_list_name.clone()))?;

                // Clone the remaining indices before selecting
                let remaining_indices = state.remaining_indices.clone();

                // Select a random index from remaining_indices
                let idx = self.rng.gen_range(0..remaining_indices.len());
                let item_idx = remaining_indices[idx];

                // Get the item
                let item = source_list.items.get(item_idx).ok_or_else(|| {
                    EvalError::EmptyList(format!("Invalid index {} in consumable list", item_idx))
                })?;

                // Remove the selected index from remaining_indices
                let mut new_remaining = remaining_indices;
                new_remaining.remove(idx);

                // Update the consumable list state
                self.consumable_lists.insert(
                    id.clone(),
                    ConsumableListState {
                        source_list_name,
                        remaining_indices: new_remaining,
                    },
                );

                // Evaluate the item
                if !item.sublists.is_empty() {
                    let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                    let sidx = self.rng.gen_range(0..sublist_names.len());
                    let sublist_name = &sublist_names[sidx];
                    let sublist = item.sublists.get(sublist_name).unwrap();
                    self.evaluate_list(sublist)
                } else {
                    self.evaluate_content(&item.content)
                }
            }
        }
    }

    fn get_property_value(&mut self, value: &Value, prop_name: &str) -> Result<Value, EvalError> {
        match value {
            Value::List(list_name) => {
                // Look up the list
                let list = self
                    .program
                    .get_list(list_name)
                    .ok_or_else(|| EvalError::UndefinedList(list_name.clone()))?;

                // Search through all items to find one with this property as a sublist
                for item in &list.items {
                    if let Some(sublist) = item.sublists.get(prop_name) {
                        return Ok(Value::ListInstance(sublist.clone()));
                    }
                }

                Err(EvalError::UndefinedProperty(
                    list_name.clone(),
                    prop_name.to_string(),
                ))
            }
            Value::ListInstance(list) => {
                // Search through all items to find one with this property
                for item in &list.items {
                    if let Some(sublist) = item.sublists.get(prop_name) {
                        return Ok(Value::ListInstance(sublist.clone()));
                    }
                }

                Err(EvalError::UndefinedProperty(
                    list.name.clone(),
                    prop_name.to_string(),
                ))
            }
            Value::Text(_) => Err(EvalError::TypeError(format!(
                "Cannot access property '{}' on text value",
                prop_name
            ))),
            Value::Array(_) => Err(EvalError::TypeError(format!(
                "Cannot access property '{}' on array value",
                prop_name
            ))),
            Value::ConsumableList(_) => Err(EvalError::TypeError(format!(
                "Cannot access property '{}' on consumable list",
                prop_name
            ))),
        }
    }

    fn get_property(&mut self, value: &Value, prop_name: &str) -> Result<String, EvalError> {
        let prop_value = self.get_property_value(value, prop_name)?;
        self.value_to_string(prop_value)
    }

    fn call_method(&mut self, value: &Value, method: &MethodCall) -> Result<String, EvalError> {
        let value_result = self.call_method_value(value, method)?;
        self.value_to_string(value_result)
    }

    fn call_method_value(
        &mut self,
        value: &Value,
        method: &MethodCall,
    ) -> Result<Value, EvalError> {
        match method.name.as_str() {
            "selectOne" => {
                // Select one item from the list and return it as a Value
                match value {
                    Value::List(name) => {
                        let list = self
                            .program
                            .get_list(name)
                            .ok_or_else(|| EvalError::UndefinedList(name.clone()))?;

                        let item = self
                            .select_weighted_item(&list.items, list.total_weight)?
                            .clone();

                        // If item has sublists, pick one randomly and return it
                        if !item.sublists.is_empty() {
                            let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                            let idx = self.rng.gen_range(0..sublist_names.len());
                            let sublist_name = &sublist_names[idx];
                            let sublist = item.sublists.get(sublist_name).unwrap();
                            return Ok(Value::ListInstance(sublist.clone()));
                        }

                        // No sublists, evaluate content directly
                        let result = self.evaluate_content(&item.content)?;
                        Ok(Value::Text(result))
                    }
                    Value::ListInstance(list) => {
                        let item = self
                            .select_weighted_item(&list.items, list.total_weight)?
                            .clone();

                        if !item.sublists.is_empty() {
                            let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                            let idx = self.rng.gen_range(0..sublist_names.len());
                            let sublist_name = &sublist_names[idx];
                            let sublist = item.sublists.get(sublist_name).unwrap();
                            return Ok(Value::ListInstance(sublist.clone()));
                        }

                        let result = self.evaluate_content(&item.content)?;
                        Ok(Value::Text(result))
                    }
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
                        let result = self.value_to_string(value.clone())?;
                        Ok(Value::Text(result))
                    }
                }
            }

            "upperCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(s.to_uppercase()))
            }

            "lowerCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(s.to_lowercase()))
            }

            "titleCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_title_case(&s)))
            }

            "sentenceCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_sentence_case(&s)))
            }

            "selectAll" => {
                // Return all items as a joined string
                match value {
                    Value::List(name) => {
                        let list = self
                            .program
                            .get_list(name)
                            .ok_or_else(|| EvalError::UndefinedList(name.clone()))?;

                        let mut results = Vec::new();
                        for item in &list.items {
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                for sublist_name in sublist_names {
                                    if let Some(sublist) = item.sublists.get(&sublist_name) {
                                        results.push(self.evaluate_list(sublist)?);
                                    }
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content)?);
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
                                        results.push(self.evaluate_list(sublist)?);
                                    }
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content)?);
                            }
                        }
                        Ok(Value::Text(results.join(" ")))
                    }
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    Value::Array(items) => {
                        // selectAll on an array just returns the array
                        Ok(Value::Array(items.clone()))
                    }
                    Value::ConsumableList(_) => {
                        // selectAll is not meaningful for consumable lists
                        Err(EvalError::InvalidMethodCall(
                            "selectAll cannot be called on consumable lists".to_string(),
                        ))
                    }
                }
            }

            "selectMany" => {
                // Select n items with repetition
                let n = if method.args.is_empty() {
                    return Err(EvalError::InvalidMethodCall(
                        "selectMany requires an argument".to_string(),
                    ));
                } else {
                    // Evaluate the argument to get the number
                    let arg_str = self.evaluate_expression(&method.args[0])?;
                    arg_str.parse::<usize>().map_err(|_| {
                        EvalError::InvalidMethodCall(format!(
                            "selectMany argument must be a number, got: {}",
                            arg_str
                        ))
                    })?
                };

                match value {
                    Value::List(name) => {
                        let list = self
                            .program
                            .get_list(name)
                            .ok_or_else(|| EvalError::UndefinedList(name.clone()))?;

                        let mut results = Vec::new();
                        for _ in 0..n {
                            let item = self
                                .select_weighted_item(&list.items, list.total_weight)?
                                .clone();
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let idx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[idx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist)?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content)?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ListInstance(list) => {
                        let mut results = Vec::new();
                        for _ in 0..n {
                            let item = self
                                .select_weighted_item(&list.items, list.total_weight)?
                                .clone();
                            if !item.sublists.is_empty() {
                                let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                                let idx = self.rng.gen_range(0..sublist_names.len());
                                let sublist_name = &sublist_names[idx];
                                if let Some(sublist) = item.sublists.get(sublist_name) {
                                    results.push(self.evaluate_list(sublist)?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content)?);
                            }
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
                        Err(EvalError::InvalidMethodCall(
                            "selectMany cannot be called on consumable lists (use selectUnique instead)".to_string(),
                        ))
                    }
                }
            }

            "selectUnique" => {
                // Select n unique items without repetition
                let n = if method.args.is_empty() {
                    return Err(EvalError::InvalidMethodCall(
                        "selectUnique requires an argument".to_string(),
                    ));
                } else {
                    let arg_str = self.evaluate_expression(&method.args[0])?;
                    arg_str.parse::<usize>().map_err(|_| {
                        EvalError::InvalidMethodCall(format!(
                            "selectUnique argument must be a number, got: {}",
                            arg_str
                        ))
                    })?
                };

                match value {
                    Value::List(name) => {
                        let list = self
                            .program
                            .get_list(name)
                            .ok_or_else(|| EvalError::UndefinedList(name.clone()))?;

                        if n > list.items.len() {
                            return Err(EvalError::InvalidMethodCall(format!(
                                "Cannot select {} unique items from list with {} items",
                                n,
                                list.items.len()
                            )));
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
                                    results.push(self.evaluate_list(sublist)?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content)?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::ListInstance(list) => {
                        if n > list.items.len() {
                            return Err(EvalError::InvalidMethodCall(format!(
                                "Cannot select {} unique items from list with {} items",
                                n,
                                list.items.len()
                            )));
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
                                    results.push(self.evaluate_list(sublist)?);
                                }
                            } else {
                                results.push(self.evaluate_content(&item.content)?);
                            }
                        }
                        Ok(Value::Array(results))
                    }
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    Value::Array(items) => {
                        // selectUnique on an array
                        if n > items.len() {
                            return Err(EvalError::InvalidMethodCall(format!(
                                "Cannot select {} unique items from array with {} items",
                                n,
                                items.len()
                            )));
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
                            let result = self.value_to_string(value.clone())?;
                            results.push(result);
                        }
                        Ok(Value::Array(results))
                    }
                }
            }

            "pluralForm" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_plural(&s)))
            }

            "pastTense" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_past_tense(&s)))
            }

            "possessiveForm" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_possessive(&s)))
            }

            "futureTense" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_future_tense(&s)))
            }

            "presentTense" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_present_tense(&s)))
            }

            "negativeForm" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_negative_form(&s)))
            }

            "singularForm" => {
                let s = self.value_to_string(value.clone())?;
                Ok(Value::Text(to_singular(&s)))
            }

            "joinItems" => {
                // Join array items with a separator
                let separator = if method.args.is_empty() {
                    " ".to_string() // Default separator
                } else {
                    self.evaluate_expression(&method.args[0])?
                };

                match value {
                    Value::Array(items) => Ok(Value::Text(items.join(&separator))),
                    Value::Text(s) => Ok(Value::Text(s.clone())),
                    _ => {
                        // Try converting to string first
                        let s = self.value_to_string(value.clone())?;
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
                            .ok_or_else(|| EvalError::UndefinedList(name.clone()))?;

                        // Generate unique ID for this consumable list
                        let id = format!("__consumable_{}__", self.consumable_list_counter);
                        self.consumable_list_counter += 1;

                        // Create list of all item indices
                        let remaining_indices: Vec<usize> = (0..list.items.len()).collect();

                        // Store the consumable list state
                        self.consumable_lists.insert(
                            id.clone(),
                            ConsumableListState {
                                source_list_name: name.clone(),
                                remaining_indices,
                            },
                        );

                        // Return reference to consumable list
                        Ok(Value::ConsumableList(id))
                    }
                    Value::ListInstance(_list) => {
                        // For list instances, we can't create a consumable version
                        // because we don't have a source list name
                        Err(EvalError::InvalidMethodCall(
                            "consumableList can only be called on named lists".to_string(),
                        ))
                    }
                    _ => Err(EvalError::InvalidMethodCall(
                        "consumableList can only be called on lists".to_string(),
                    )),
                }
            }

            _ => Err(EvalError::InvalidMethodCall(format!(
                "Unknown method: {}",
                method.name
            ))),
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

pub fn evaluate<R: Rng>(program: &CompiledProgram, rng: &mut R) -> Result<String, EvalError> {
    let mut evaluator = Evaluator::new(program, rng);
    evaluator.evaluate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::parser::parse;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_simple_evaluation() {
        let input = "output\n\thello world\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_list_reference() {
        let input = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a [animal].\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output == "I saw a dog." || output == "I saw a cat.");
    }

    #[test]
    fn test_deterministic() {
        let input = "animal\n\tdog\n\tcat\n\noutput\n\t[animal]\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();

        let mut rng1 = StdRng::seed_from_u64(12345);
        let result1 = evaluate(&compiled, &mut rng1).unwrap();

        let mut rng2 = StdRng::seed_from_u64(12345);
        let result2 = evaluate(&compiled, &mut rng2).unwrap();

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_inline_list() {
        let input = "output\n\t{big|small}\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output == "big" || output == "small");
    }

    #[test]
    fn test_number_range() {
        let input = "output\n\t{1-6}\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng);
        assert!(result.is_ok());
        let output = result.unwrap();
        let num: i32 = output.parse().unwrap();
        assert!((1..=6).contains(&num));
    }
}
