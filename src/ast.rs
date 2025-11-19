/// Abstract Syntax Tree definitions for Perchance language
use crate::span::Spanned;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub lists: Vec<Spanned<List>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub name: String,
    pub items: Vec<Spanned<Item>>,
    pub output: Option<Vec<Spanned<ContentPart>>>, // $output property
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemWeight {
    Static(f64),
    Dynamic(Box<Spanned<Expression>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub content: Vec<Spanned<ContentPart>>,
    pub weight: Option<ItemWeight>,
    pub sublists: Vec<Spanned<List>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentPart {
    Text(String),
    Reference(Spanned<Expression>),
    Inline(Spanned<InlineList>),
    Escape(char),
    // Special inline functions
    Article,   // {a} - outputs "a" or "an" based on next word
    Pluralize, // {s} - outputs "s" for plural or "" for singular based on previous number
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineList {
    pub choices: Vec<Spanned<InlineChoice>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineChoice {
    pub content: Vec<Spanned<ContentPart>>,
    pub weight: Option<ItemWeight>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Simple reference: [animal]
    Simple(Spanned<Identifier>),

    // Property access: [animal.name]
    Property(Box<Spanned<Expression>>, Spanned<Identifier>),

    // Property access with fallback: [animal.name || "default"]
    PropertyWithFallback(
        Box<Spanned<Expression>>,
        Spanned<Identifier>,
        Box<Spanned<Expression>>,
    ),

    // Dynamic access: [animal[x]]
    Dynamic(Box<Spanned<Expression>>, Box<Spanned<Expression>>),

    // Method call: [animal.selectOne]
    Method(Box<Spanned<Expression>>, Spanned<MethodCall>),

    // Assignment: [x = animal]
    Assignment(Spanned<Identifier>, Box<Spanned<Expression>>),

    // Property assignment: [this.property = value]
    PropertyAssignment(
        Box<Spanned<Expression>>,
        Spanned<Identifier>,
        Box<Spanned<Expression>>,
    ),

    // Multiple statements with comma: [x = animal, y = color, "result"]
    Sequence(Vec<Spanned<Expression>>, Option<Box<Spanned<Expression>>>),

    // String literal: "hello"
    Literal(String),

    // Number literal: 42 or 3.14
    Number(f64),

    // Number range: {1-10}
    NumberRange(i64, i64),

    // Letter range: {a-z}
    LetterRange(char, char),

    // Conditional: condition ? trueExpr : falseExpr
    Conditional(
        Box<Spanned<Expression>>,
        Box<Spanned<Expression>>,
        Box<Spanned<Expression>>,
    ),

    // Long-form if/else: if (cond) {expr} else if (cond) {expr} else {expr}
    IfElse {
        condition: Box<Spanned<Expression>>,
        then_expr: Box<Spanned<Expression>>,
        else_expr: Option<Box<Spanned<Expression>>>, // Can be another IfElse for chaining
    },

    // Repeat: repeat(n) {expr}
    Repeat {
        count: Box<Spanned<Expression>>,
        body: Box<Spanned<Expression>>,
    },

    // Binary operations: ==, !=, <, >, <=, >=, &&, ||
    BinaryOp(
        Box<Spanned<Expression>>,
        BinaryOperator,
        Box<Spanned<Expression>>,
    ),

    // Import: {import:generator-name}
    Import(String),
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
    pub args: Vec<Spanned<Expression>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

impl Program {
    pub fn new() -> Self {
        Program { lists: Vec::new() }
    }

    pub fn add_list(&mut self, list: Spanned<List>) {
        self.lists.push(list);
    }

    pub fn find_list(&self, name: &str) -> Option<&Spanned<List>> {
        self.lists.iter().find(|l| l.value.name == name)
    }
}

impl List {
    pub fn new(name: String) -> Self {
        List {
            name,
            items: Vec::new(),
            output: None,
        }
    }

    pub fn add_item(&mut self, item: Spanned<Item>) {
        self.items.push(item);
    }

    pub fn set_output(&mut self, output: Vec<Spanned<ContentPart>>) {
        self.output = Some(output);
    }
}

impl Item {
    pub fn new(content: Vec<Spanned<ContentPart>>) -> Self {
        Item {
            content,
            weight: None,
            sublists: Vec::new(),
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

    pub fn with_dynamic_weight(mut self, expr: Spanned<Expression>) -> Self {
        self.weight = Some(ItemWeight::Dynamic(Box::new(expr)));
        self
    }

    pub fn add_sublist(&mut self, list: Spanned<List>) {
        self.sublists.push(list);
    }
}

impl InlineList {
    pub fn new(choices: Vec<Spanned<InlineChoice>>) -> Self {
        InlineList { choices }
    }
}

impl InlineChoice {
    pub fn new(content: Vec<Spanned<ContentPart>>) -> Self {
        InlineChoice {
            content,
            weight: None,
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
        Identifier { name }
    }
}

impl MethodCall {
    pub fn new(name: String) -> Self {
        MethodCall {
            name,
            args: Vec::new(),
        }
    }

    pub fn with_args(mut self, args: Vec<Spanned<Expression>>) -> Self {
        self.args = args;
        self
    }
}
