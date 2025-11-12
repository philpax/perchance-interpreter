/// Compiler transforms AST into an evaluatable representation
use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompiledProgram {
    pub lists: HashMap<String, CompiledList>,
}

#[derive(Debug, Clone)]
pub struct CompiledList {
    pub name: String,
    pub items: Vec<CompiledItem>,
    pub total_weight: f64,
    pub output: Option<Vec<ContentPart>>, // $output property
}

#[derive(Debug, Clone)]
pub struct CompiledItem {
    pub content: Vec<ContentPart>,
    pub weight: f64,
    pub sublists: HashMap<String, CompiledList>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    UndefinedList(String),
    EmptyList(String),
    DuplicateList(String),
    InvalidWeight(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UndefinedList(name) => write!(f, "Undefined list: {}", name),
            CompileError::EmptyList(name) => write!(f, "Empty list: {}", name),
            CompileError::DuplicateList(name) => write!(f, "Duplicate list name: {}", name),
            CompileError::InvalidWeight(msg) => write!(f, "Invalid weight: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

impl Default for CompiledProgram {
    fn default() -> Self {
        Self::new()
    }
}

impl CompiledProgram {
    pub fn new() -> Self {
        CompiledProgram {
            lists: HashMap::new(),
        }
    }

    pub fn add_list(&mut self, name: String, list: CompiledList) {
        self.lists.insert(name, list);
    }

    pub fn get_list(&self, name: &str) -> Option<&CompiledList> {
        self.lists.get(name)
    }
}

impl CompiledList {
    pub fn new(name: String) -> Self {
        CompiledList {
            name,
            items: Vec::new(),
            total_weight: 0.0,
            output: None,
        }
    }

    pub fn add_item(&mut self, item: CompiledItem) {
        self.total_weight += item.weight;
        self.items.push(item);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl CompiledItem {
    pub fn new(content: Vec<ContentPart>, weight: f64) -> Self {
        CompiledItem {
            content,
            weight,
            sublists: HashMap::new(),
        }
    }

    pub fn add_sublist(&mut self, name: String, list: CompiledList) {
        self.sublists.insert(name, list);
    }
}

pub fn compile(program: &Program) -> Result<CompiledProgram, CompileError> {
    let mut compiled = CompiledProgram::new();

    // First pass: compile all lists
    for list in &program.lists {
        if compiled.lists.contains_key(&list.name) {
            return Err(CompileError::DuplicateList(list.name.clone()));
        }

        let compiled_list = compile_list(list)?;
        compiled.add_list(list.name.clone(), compiled_list);
    }

    Ok(compiled)
}

fn compile_list(list: &List) -> Result<CompiledList, CompileError> {
    if list.items.is_empty() && list.output.is_none() {
        return Err(CompileError::EmptyList(list.name.clone()));
    }

    let mut compiled_list = CompiledList::new(list.name.clone());

    for item in &list.items {
        let compiled_item = compile_item(item)?;
        compiled_list.add_item(compiled_item);
    }

    // Copy the output field
    compiled_list.output = list.output.clone();

    Ok(compiled_list)
}

fn compile_item(item: &Item) -> Result<CompiledItem, CompileError> {
    let weight = item.weight.unwrap_or(1.0);

    if weight < 0.0 {
        return Err(CompileError::InvalidWeight(
            "Weight cannot be negative".to_string(),
        ));
    }

    let mut compiled_item = CompiledItem::new(item.content.clone(), weight);

    // Compile sublists
    for sublist in &item.sublists {
        let compiled_sublist = compile_list(sublist)?;
        compiled_item.add_sublist(sublist.name.clone(), compiled_sublist);
    }

    Ok(compiled_item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_compile_simple() {
        let input = "animal\n\tdog\n\tcat\n";
        let program = parse(input).unwrap();
        let result = compile(&program);
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert_eq!(compiled.lists.len(), 1);
        assert!(compiled.get_list("animal").is_some());
    }

    #[test]
    fn test_compile_with_weights() {
        let input = "animal\n\tdog^2\n\tcat^0.5\n";
        let program = parse(input).unwrap();
        let result = compile(&program);
        assert!(result.is_ok());
        let compiled = result.unwrap();
        let animal_list = compiled.get_list("animal").unwrap();
        assert_eq!(animal_list.total_weight, 2.5);
    }

    #[test]
    fn test_empty_list_error() {
        let mut program = Program::new();
        program.add_list(List::new("empty".to_string()));
        let result = compile(&program);
        assert!(result.is_err());
    }
}
