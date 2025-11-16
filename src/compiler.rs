/// Compiler transforms AST into an evaluatable representation
use crate::ast::*;
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompiledProgram {
    pub lists: HashMap<String, CompiledList>,
    pub list_order: Vec<String>, // Preserve order of lists for default output
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
    pub weight: f64, // For static weights and as default for dynamic ones
    pub dynamic_weight: Option<Expression>, // For dynamic weights like ^[condition]
    pub sublists: HashMap<String, CompiledList>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    UndefinedList { name: String, span: Span },
    EmptyList { name: String, span: Span },
    DuplicateList { name: String, span: Span },
    InvalidWeight { message: String, span: Span },
}

impl CompileError {
    pub fn span(&self) -> Span {
        match self {
            CompileError::UndefinedList { span, .. } => *span,
            CompileError::EmptyList { span, .. } => *span,
            CompileError::DuplicateList { span, .. } => *span,
            CompileError::InvalidWeight { span, .. } => *span,
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UndefinedList { name, .. } => write!(f, "Undefined list: {}", name),
            CompileError::EmptyList { name, .. } => write!(f, "Empty list: {}", name),
            CompileError::DuplicateList { name, .. } => write!(f, "Duplicate list name: {}", name),
            CompileError::InvalidWeight { message, .. } => write!(f, "Invalid weight: {}", message),
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
            list_order: Vec::new(),
        }
    }

    pub fn add_list(&mut self, name: String, list: CompiledList) {
        self.list_order.push(name.clone());
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
            dynamic_weight: None,
            sublists: HashMap::new(),
        }
    }

    pub fn new_with_dynamic_weight(content: Vec<ContentPart>, dynamic_weight: Expression) -> Self {
        CompiledItem {
            content,
            weight: 0.0, // Will be calculated at runtime
            dynamic_weight: Some(dynamic_weight),
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
            return Err(CompileError::DuplicateList {
                name: list.name.clone(),
                span: list.span,
            });
        }

        let compiled_list = compile_list(list)?;
        compiled.add_list(list.name.clone(), compiled_list);
    }

    Ok(compiled)
}

fn compile_list(list: &List) -> Result<CompiledList, CompileError> {
    if list.items.is_empty() && list.output.is_none() {
        return Err(CompileError::EmptyList {
            name: list.name.clone(),
            span: list.span,
        });
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
    let mut compiled_item = match &item.weight {
        Some(ItemWeight::Static(w)) => {
            if *w < 0.0 {
                return Err(CompileError::InvalidWeight {
                    message: "Weight cannot be negative".to_string(),
                    span: item.span,
                });
            }
            CompiledItem::new(item.content.clone(), *w)
        }
        Some(ItemWeight::Dynamic(expr)) => {
            CompiledItem::new_with_dynamic_weight(item.content.clone(), *expr.clone())
        }
        None => {
            // Default weight is 1.0
            CompiledItem::new(item.content.clone(), 1.0)
        }
    };

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
        let compiled = result.unwrap();
        assert_eq!(compiled.lists.len(), 1);
        assert!(compiled.get_list("animal").is_some());
    }

    #[test]
    fn test_compile_with_weights() {
        let input = "animal\n\tdog^2\n\tcat^0.5\n";
        let program = parse(input).unwrap();
        let result = compile(&program);
        let compiled = result.unwrap();
        let animal_list = compiled.get_list("animal").unwrap();
        assert_eq!(animal_list.total_weight, 2.5);
    }

    #[test]
    fn test_empty_list_error() {
        let mut program = Program::new();
        program.add_list(List::new("empty".to_string()));
        let result = compile(&program);
        result.unwrap_err();
    }
}
