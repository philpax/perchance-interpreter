/// Abstract Syntax Tree definitions for Perchance language
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub lists: Vec<List>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub name: String,
    pub items: Vec<Item>,
    pub output: Option<Vec<ContentPart>>, // $output property
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemWeight {
    Static(f64),
    Dynamic(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub content: Vec<ContentPart>,
    pub weight: Option<ItemWeight>,
    pub sublists: Vec<List>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentPart {
    Text(String, Span),
    Reference(Expression, Span),
    Inline(InlineList, Span),
    Escape(char, Span),
    // Special inline functions
    Article(Span),   // {a} - outputs "a" or "an" based on next word
    Pluralize(Span), // {s} - outputs "s" for plural or "" for singular based on previous number
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineList {
    pub choices: Vec<InlineChoice>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineChoice {
    pub content: Vec<ContentPart>,
    pub weight: Option<ItemWeight>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Simple reference: [animal]
    Simple(Identifier, Span),

    // Property access: [animal.name]
    Property(Box<Expression>, Identifier, Span),

    // Property access with fallback: [animal.name || "default"]
    PropertyWithFallback(Box<Expression>, Identifier, Box<Expression>, Span),

    // Dynamic access: [animal[x]]
    Dynamic(Box<Expression>, Box<Expression>, Span),

    // Method call: [animal.selectOne]
    Method(Box<Expression>, MethodCall, Span),

    // Assignment: [x = animal]
    Assignment(Identifier, Box<Expression>, Span),

    // Property assignment: [this.property = value]
    PropertyAssignment(Box<Expression>, Identifier, Box<Expression>, Span),

    // Multiple statements with comma: [x = animal, y = color, "result"]
    Sequence(Vec<Expression>, Option<Box<Expression>>, Span),

    // String literal: "hello"
    Literal(String, Span),

    // Number literal: 42 or 3.14
    Number(f64, Span),

    // Number range: {1-10}
    NumberRange(i64, i64, Span),

    // Letter range: {a-z}
    LetterRange(char, char, Span),

    // Conditional: condition ? trueExpr : falseExpr
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>, Span),

    // Binary operations: ==, !=, <, >, <=, >=, &&, ||
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>, Span),

    // Import: {import:generator-name}
    Import(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    // Comparison operators
    Equal,        // ==
    NotEqual,     // !=
    LessThan,     // <
    GreaterThan,  // >
    LessEqual,    // <=
    GreaterEqual, // >=
    // Logical operators
    And, // &&
    Or,  // ||
    // Math operators
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCall {
    pub name: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

impl Program {
    pub fn new() -> Self {
        Program {
            lists: Vec::new(),
            span: Span::dummy(),
        }
    }

    pub fn add_list(&mut self, list: List) {
        self.lists.push(list);
    }

    pub fn find_list(&self, name: &str) -> Option<&List> {
        self.lists.iter().find(|l| l.name == name)
    }
}

impl List {
    pub fn new(name: String) -> Self {
        List {
            name,
            items: Vec::new(),
            output: None,
            span: Span::dummy(),
        }
    }

    pub fn new_with_span(name: String, span: Span) -> Self {
        List {
            name,
            items: Vec::new(),
            output: None,
            span,
        }
    }

    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn set_output(&mut self, output: Vec<ContentPart>) {
        self.output = Some(output);
    }
}

impl Item {
    pub fn new(content: Vec<ContentPart>) -> Self {
        Item {
            content,
            weight: None,
            sublists: Vec::new(),
            span: Span::dummy(),
        }
    }

    pub fn new_with_span(content: Vec<ContentPart>, span: Span) -> Self {
        Item {
            content,
            weight: None,
            sublists: Vec::new(),
            span,
        }
    }

    pub fn with_weight(mut self, weight: ItemWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn with_static_weight(mut self, weight: f64) -> Self {
        self.weight = Some(ItemWeight::Static(weight));
        self
    }

    pub fn with_dynamic_weight(mut self, expr: Expression) -> Self {
        self.weight = Some(ItemWeight::Dynamic(Box::new(expr)));
        self
    }

    pub fn add_sublist(&mut self, list: List) {
        self.sublists.push(list);
    }
}

impl InlineList {
    pub fn new(choices: Vec<InlineChoice>) -> Self {
        InlineList {
            choices,
            span: Span::dummy(),
        }
    }

    pub fn new_with_span(choices: Vec<InlineChoice>, span: Span) -> Self {
        InlineList { choices, span }
    }
}

impl InlineChoice {
    pub fn new(content: Vec<ContentPart>) -> Self {
        InlineChoice {
            content,
            weight: None,
            span: Span::dummy(),
        }
    }

    pub fn new_with_span(content: Vec<ContentPart>, span: Span) -> Self {
        InlineChoice {
            content,
            weight: None,
            span,
        }
    }

    pub fn with_weight(mut self, weight: ItemWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn with_static_weight(mut self, weight: f64) -> Self {
        self.weight = Some(ItemWeight::Static(weight));
        self
    }
}

impl Identifier {
    pub fn new(name: String) -> Self {
        Identifier {
            name,
            span: Span::dummy(),
        }
    }

    pub fn new_with_span(name: String, span: Span) -> Self {
        Identifier { name, span }
    }
}

impl MethodCall {
    pub fn new(name: String) -> Self {
        MethodCall {
            name,
            args: Vec::new(),
            span: Span::dummy(),
        }
    }

    pub fn new_with_span(name: String, span: Span) -> Self {
        MethodCall {
            name,
            args: Vec::new(),
            span,
        }
    }

    pub fn with_args(mut self, args: Vec<Expression>) -> Self {
        self.args = args;
        self
    }
}
