/// Abstract Syntax Tree definitions for Perchance language

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub lists: Vec<List>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub name: String,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub content: Vec<ContentPart>,
    pub weight: Option<f64>,
    pub sublists: Vec<List>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentPart {
    Text(String),
    Reference(Expression),
    Inline(InlineList),
    Escape(char),
    // Special inline functions
    Article,    // {a} - outputs "a" or "an" based on next word
    Pluralize,  // {s} - outputs "s" for plural or "" for singular based on previous number
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineList {
    pub choices: Vec<InlineChoice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineChoice {
    pub content: Vec<ContentPart>,
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Simple reference: [animal]
    Simple(Identifier),

    // Property access: [animal.name]
    Property(Box<Expression>, Identifier),

    // Dynamic access: [animal[x]]
    Dynamic(Box<Expression>, Box<Expression>),

    // Method call: [animal.selectOne]
    Method(Box<Expression>, MethodCall),

    // Assignment: [x = animal]
    Assignment(Identifier, Box<Expression>),

    // Multiple statements with comma: [x = animal, y = color, "result"]
    Sequence(Vec<Expression>, Option<Box<Expression>>),

    // String literal: "hello"
    Literal(String),

    // Number range: {1-10}
    NumberRange(i64, i64),

    // Letter range: {a-z}
    LetterRange(char, char),

    // Conditional: condition ? trueExpr : falseExpr
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),

    // Binary operations: ==, !=, <, >, <=, >=, &&, ||
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Equal,        // ==
    NotEqual,     // !=
    LessThan,     // <
    GreaterThan,  // >
    LessEqual,    // <=
    GreaterEqual, // >=
    And,          // &&
    Or,           // ||
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCall {
    pub name: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
}

impl Program {
    pub fn new() -> Self {
        Program { lists: Vec::new() }
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
        }
    }

    pub fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }
}

impl Item {
    pub fn new(content: Vec<ContentPart>) -> Self {
        Item {
            content,
            weight: None,
            sublists: Vec::new(),
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn add_sublist(&mut self, list: List) {
        self.sublists.push(list);
    }
}

impl InlineList {
    pub fn new(choices: Vec<InlineChoice>) -> Self {
        InlineList { choices }
    }
}

impl InlineChoice {
    pub fn new(content: Vec<ContentPart>) -> Self {
        InlineChoice {
            content,
            weight: None,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
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

    pub fn with_args(mut self, args: Vec<Expression>) -> Self {
        self.args = args;
        self
    }
}
