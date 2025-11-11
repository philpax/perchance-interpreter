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
enum Value {
    Text(String),
    List(String), // Reference to a list by name
    ListInstance(CompiledList), // An actual list instance (for sublists)
}

pub struct Evaluator<'a, R: Rng> {
    program: &'a CompiledProgram,
    rng: &'a mut R,
    variables: HashMap<String, Value>,
}

impl<'a, R: Rng> Evaluator<'a, R> {
    pub fn new(program: &'a CompiledProgram, rng: &'a mut R) -> Self {
        Evaluator {
            program,
            rng,
            variables: HashMap::new(),
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
        if list.items.is_empty() {
            return Err(EvalError::EmptyList(list.name.clone()));
        }

        // Select an item based on weights
        let item = self.select_weighted_item(&list.items, list.total_weight)?.clone();

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

        for part in content {
            match part {
                ContentPart::Text(text) => result.push_str(text),
                ContentPart::Escape(ch) => result.push(*ch),
                ContentPart::Reference(expr) => {
                    let value = self.evaluate_expression(expr)?;
                    result.push_str(&value);
                }
                ContentPart::Inline(inline) => {
                    let value = self.evaluate_inline(inline)?;
                    result.push_str(&value);
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
        let total_weight: f64 = inline
            .choices
            .iter()
            .map(|c| c.weight.unwrap_or(1.0))
            .sum();

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

    fn evaluate_expression(&mut self, expr: &Expression) -> Result<String, EvalError> {
        match expr {
            Expression::Simple(ident) => {
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
                let base_value = self.evaluate_to_value(base)?;
                self.get_property(&base_value, &prop.name)
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
        }
    }

    fn evaluate_to_value(&mut self, expr: &Expression) -> Result<Value, EvalError> {
        match expr {
            Expression::Simple(ident) => {
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
                let base_value = self.evaluate_to_value(base)?;
                // For properties, we need to get the property and return it as a value
                let prop_str = self.get_property(&base_value, &prop.name)?;
                Ok(Value::Text(prop_str))
            }

            Expression::Method(base, method) => {
                let base_value = self.evaluate_to_value(base)?;
                let result = self.call_method(&base_value, method)?;
                Ok(Value::Text(result))
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
        }
    }

    fn get_property(&mut self, value: &Value, prop_name: &str) -> Result<String, EvalError> {
        match value {
            Value::List(list_name) => {
                // Look up the list
                let list = self
                    .program
                    .get_list(list_name)
                    .ok_or_else(|| EvalError::UndefinedList(list_name.clone()))?;

                // Check if any item has this property as a sublist
                // For simplicity, we'll select an item and look for the property
                let item = self.select_weighted_item(&list.items, list.total_weight)?.clone();

                if let Some(sublist) = item.sublists.get(prop_name) {
                    return self.evaluate_list(sublist);
                }

                Err(EvalError::UndefinedProperty(
                    list_name.clone(),
                    prop_name.to_string(),
                ))
            }
            Value::ListInstance(list) => {
                // Look for property in the list items
                let item = self.select_weighted_item(&list.items, list.total_weight)?.clone();

                if let Some(sublist) = item.sublists.get(prop_name) {
                    return self.evaluate_list(sublist);
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
        }
    }

    fn call_method(&mut self, value: &Value, method: &MethodCall) -> Result<String, EvalError> {
        match method.name.as_str() {
            "selectOne" => {
                // Select one item from the list
                match value {
                    Value::List(name) => {
                        let list = self
                            .program
                            .get_list(name)
                            .ok_or_else(|| EvalError::UndefinedList(name.clone()))?;
                        // Select an item and store it as a ListInstance
                        let item =
                            self.select_weighted_item(&list.items, list.total_weight)?.clone();

                        // If item has sublists, pick one
                        if !item.sublists.is_empty() {
                            let sublist_names: Vec<_> = item.sublists.keys().cloned().collect();
                            let idx = self.rng.gen_range(0..sublist_names.len());
                            let sublist_name = &sublist_names[idx];
                            let sublist = item.sublists.get(sublist_name).unwrap();
                            return self.evaluate_list(sublist);
                        }

                        self.evaluate_content(&item.content)
                    }
                    Value::ListInstance(list) => self.evaluate_list(list),
                    Value::Text(s) => Ok(s.clone()),
                }
            }

            "upperCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(s.to_uppercase())
            }

            "lowerCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(s.to_lowercase())
            }

            "titleCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(to_title_case(&s))
            }

            "sentenceCase" => {
                let s = self.value_to_string(value.clone())?;
                Ok(to_sentence_case(&s))
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

pub fn evaluate<R: Rng>(
    program: &CompiledProgram,
    rng: &mut R,
) -> Result<String, EvalError> {
    let mut evaluator = Evaluator::new(program, rng);
    evaluator.evaluate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::parser::parse;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

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
        assert!(num >= 1 && num <= 6);
    }
}
