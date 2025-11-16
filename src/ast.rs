/// Abstract Syntax Tree definitions for Perchance language

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub lists: Vec<List>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub name: String,
    pub items: Vec<Item>,
    pub output: Option<Vec<ContentPart>>, // $output property
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentPart {
    Text(String),
    Reference(Expression),
    Inline(InlineList),
    Escape(char),
    // Special inline functions
    Article,   // {a} - outputs "a" or "an" based on next word
    Pluralize, // {s} - outputs "s" for plural or "" for singular based on previous number
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineList {
    pub choices: Vec<InlineChoice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineChoice {
    pub content: Vec<ContentPart>,
    pub weight: Option<ItemWeight>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // Simple reference: [animal]
    Simple(Identifier),

    // Property access: [animal.name]
    Property(Box<Expression>, Identifier),

    // Property access with fallback: [animal.name || "default"]
    PropertyWithFallback(Box<Expression>, Identifier, Box<Expression>),

    // Dynamic access: [animal[x]]
    Dynamic(Box<Expression>, Box<Expression>),

    // Method call: [animal.selectOne]
    Method(Box<Expression>, MethodCall),

    // Assignment: [x = animal]
    Assignment(Identifier, Box<Expression>),

    // Property assignment: [this.property = value]
    PropertyAssignment(Box<Expression>, Identifier, Box<Expression>),

    // Multiple statements with comma: [x = animal, y = color, "result"]
    Sequence(Vec<Expression>, Option<Box<Expression>>),

    // String literal: "hello"
    Literal(String),

    // Number literal: 42 or 3.14
    Number(f64),

    // Number range: {1-10}
    NumberRange(i64, i64),

    // Letter range: {a-z}
    LetterRange(char, char),

    // Conditional: condition ? trueExpr : falseExpr
    Conditional(Box<Expression>, Box<Expression>, Box<Expression>),

    // Binary operations: ==, !=, <, >, <=, >=, &&, ||
    BinaryOp(Box<Expression>, BinaryOperator, Box<Expression>),

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
    pub args: Vec<Expression>,
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

    pub fn with_args(mut self, args: Vec<Expression>) -> Self {
        self.args = args;
        self
    }
}
