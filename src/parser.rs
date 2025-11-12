/// Parser for Perchance language
use crate::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEof,
    InvalidIndentation(usize),
    InvalidSyntax(String, usize),
    UnterminatedReference(usize),
    UnterminatedInline(usize),
    UnterminatedString(usize),
    InvalidEscape(char, usize),
    InvalidNumberRange(usize),
    EmptyListName(usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "Unexpected end of file"),
            ParseError::InvalidIndentation(line) => {
                write!(f, "Invalid indentation at line {}", line)
            }
            ParseError::InvalidSyntax(msg, line) => write!(f, "{} at line {}", msg, line),
            ParseError::UnterminatedReference(line) => {
                write!(f, "Unterminated reference at line {}", line)
            }
            ParseError::UnterminatedInline(line) => {
                write!(f, "Unterminated inline list at line {}", line)
            }
            ParseError::UnterminatedString(line) => {
                write!(f, "Unterminated string at line {}", line)
            }
            ParseError::InvalidEscape(ch, line) => {
                write!(f, "Invalid escape sequence '\\{}' at line {}", ch, line)
            }
            ParseError::InvalidNumberRange(line) => {
                write!(f, "Invalid number range at line {}", line)
            }
            ParseError::EmptyListName(line) => write!(f, "Empty list name at line {}", line),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    input: Vec<char>,
    pos: usize,
    line: usize,
    space_indent_unit: Option<usize>, // Detected space indentation unit (2 or 4 spaces)
}

impl Parser {
    pub fn new(input: &str) -> Self {
        Parser {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            space_indent_unit: None,
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut program = Program::new();

        while !self.is_eof() {
            self.skip_whitespace_and_comments()?;
            if self.is_eof() {
                break;
            }

            // Only parse lists at the top level (no indentation)
            if self.get_indentation_level() == 0 {
                let list = self.parse_list(0)?;
                program.add_list(list);
            } else {
                return Err(ParseError::InvalidIndentation(self.line));
            }
        }

        Ok(program)
    }

    fn parse_list(&mut self, expected_indent: usize) -> Result<List, ParseError> {
        // Parse list name
        let name = self.parse_identifier()?;
        if name.is_empty() {
            return Err(ParseError::EmptyListName(self.line));
        }

        self.skip_spaces();

        // Check if this is a direct assignment: listname = expression
        if self.peek_char() == Some('=') {
            self.consume_char('=');
            self.skip_spaces();

            // Parse the expression until newline
            let output_content = self.parse_content_until_newline()?;
            self.skip_to_newline();
            self.consume_char('\n');

            // Create a list with just the output property set
            let mut list = List::new(name);
            list.set_output(output_content);
            return Ok(list);
        }

        self.skip_to_newline();
        self.consume_char('\n');

        let mut list = List::new(name);
        let item_indent = expected_indent + 1;

        // Parse items
        while !self.is_eof() {
            self.skip_empty_lines();
            if self.is_eof() {
                break;
            }

            let indent = self.get_indentation_level();

            if indent < item_indent {
                // End of this list
                break;
            } else if indent == item_indent {
                // Parse item
                self.skip_indent(item_indent);

                // Check if this is a $output line
                if self.peek_identifier() == "$output" {
                    let _ = self.parse_identifier(); // consume "$output"
                    self.skip_spaces();

                    // Expect = sign
                    if self.peek_char() == Some('=') {
                        self.consume_char('=');
                        self.skip_spaces();

                        // Parse the output content
                        let output_content = self.parse_content_until_newline()?;
                        list.set_output(output_content);

                        self.skip_to_newline();
                        if !self.is_eof() {
                            self.consume_char('\n');
                        }
                    } else {
                        return Err(ParseError::InvalidSyntax(
                            "Expected '=' after $output".to_string(),
                            self.line,
                        ));
                    }
                } else {
                    let item = self.parse_item(item_indent)?;
                    list.add_item(item);
                }
            } else {
                // Too much indentation
                return Err(ParseError::InvalidIndentation(self.line));
            }
        }

        Ok(list)
    }

    fn parse_item(&mut self, expected_indent: usize) -> Result<Item, ParseError> {
        // Parse item content until newline or weight
        let content = self.parse_content_until_newline()?;

        // Check for weight (^number)
        let weight = if self.peek_char() == Some('^') {
            self.consume_char('^');
            Some(self.parse_number()?)
        } else {
            None
        };

        self.skip_to_newline();
        self.consume_char('\n');

        let mut item = Item::new(content.clone());
        if let Some(w) = weight {
            item = item.with_weight(w);
        }

        // Check if content is just a simple identifier (potential sublist name)
        let simple_name = if content.len() == 1 {
            if let ContentPart::Text(ref s) = content[0] {
                // Check if it's a valid identifier (letters, numbers, underscore only)
                if s.chars().all(|c| c.is_alphanumeric() || c == '_') && !s.is_empty() {
                    Some(s.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Parse sublists
        // If we have a simple name and indented content, treat it as a sublist
        if let Some(sublist_name) = simple_name {
            // Check if there are indented items
            self.skip_empty_lines();
            if !self.is_eof() && self.get_indentation_level() == expected_indent + 1 {
                // Parse all the indented items as a single sublist
                let mut sublist = List::new(sublist_name);
                let sublist_indent = expected_indent + 1;

                while !self.is_eof() {
                    self.skip_empty_lines();
                    if self.is_eof() {
                        break;
                    }

                    let indent = self.get_indentation_level();
                    if indent < sublist_indent {
                        break;
                    } else if indent == sublist_indent {
                        self.skip_indent(sublist_indent);
                        let subitem = self.parse_item(sublist_indent)?;
                        sublist.add_item(subitem);
                    } else {
                        // Deeper nesting belongs to the subitem
                        break;
                    }
                }

                // Clear the content and add the sublist
                item.content.clear();
                item.add_sublist(sublist);
            }
        }

        Ok(item)
    }

    pub fn parse_content_until_newline(&mut self) -> Result<Vec<ContentPart>, ParseError> {
        let mut parts = Vec::new();
        let mut text_buffer = String::new();

        while let Some(&ch) = self.peek_char_ref() {
            match ch {
                '\n' | '\r' => break,
                '/' if self.peek_ahead(1) == Some('/') => break, // Comment
                '^' => break,                                    // Weight marker
                '\\' => {
                    // Escape sequence
                    if !text_buffer.is_empty() {
                        parts.push(ContentPart::Text(text_buffer.clone()));
                        text_buffer.clear();
                    }
                    self.consume_char('\\');
                    let escaped = self.parse_escape()?;
                    parts.push(ContentPart::Escape(escaped));
                }
                '[' => {
                    // Reference
                    if !text_buffer.is_empty() {
                        parts.push(ContentPart::Text(text_buffer.clone()));
                        text_buffer.clear();
                    }
                    let expr = self.parse_reference()?;
                    parts.push(ContentPart::Reference(expr));
                }
                '{' => {
                    // Inline list or number range
                    if !text_buffer.is_empty() {
                        parts.push(ContentPart::Text(text_buffer.clone()));
                        text_buffer.clear();
                    }
                    let inline = self.parse_inline()?;
                    parts.push(ContentPart::Inline(inline));
                }
                _ => {
                    text_buffer.push(ch);
                    self.advance();
                }
            }
        }

        if !text_buffer.is_empty() {
            parts.push(ContentPart::Text(text_buffer));
        }

        Ok(parts)
    }

    fn parse_reference(&mut self) -> Result<Expression, ParseError> {
        let start_line = self.line;
        self.consume_char('[');

        let expr = self.parse_expression()?;

        if self.peek_char() != Some(']') {
            return Err(ParseError::UnterminatedReference(start_line));
        }
        self.consume_char(']');

        Ok(expr)
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.skip_spaces();

        // Check for comma-separated sequences first
        let first = self.parse_ternary_expression()?;

        self.skip_spaces();
        if self.peek_char() == Some(',') {
            let mut exprs = vec![first];

            while self.peek_char() == Some(',') {
                self.consume_char(',');
                self.skip_spaces();

                exprs.push(self.parse_ternary_expression()?);
                self.skip_spaces();
            }

            // The last expression is the output
            let output = exprs.pop();
            Ok(Expression::Sequence(exprs, output.map(Box::new)))
        } else {
            Ok(first)
        }
    }

    fn parse_ternary_expression(&mut self) -> Result<Expression, ParseError> {
        // Parse ternary conditional: condition ? true_expr : false_expr
        let first = self.parse_or_expression()?;

        self.skip_spaces();
        if self.peek_char() == Some('?') {
            self.consume_char('?');
            self.skip_spaces();
            let true_expr = self.parse_or_expression()?;
            self.skip_spaces();

            if self.peek_char() != Some(':') {
                return Err(ParseError::InvalidSyntax(
                    "Expected ':' in ternary expression".to_string(),
                    self.line,
                ));
            }
            self.consume_char(':');
            self.skip_spaces();

            let false_expr = self.parse_ternary_expression()?;
            return Ok(Expression::Conditional(
                Box::new(first),
                Box::new(true_expr),
                Box::new(false_expr),
            ));
        }

        Ok(first)
    }

    fn parse_or_expression(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and_expression()?;

        loop {
            self.skip_spaces();
            if self.peek_two_chars() == Some(('|', '|')) {
                self.advance();
                self.advance();
                self.skip_spaces();
                let right = self.parse_and_expression()?;
                left = Expression::BinaryOp(Box::new(left), BinaryOperator::Or, Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison_expression()?;

        loop {
            self.skip_spaces();
            if self.peek_two_chars() == Some(('&', '&')) {
                self.advance();
                self.advance();
                self.skip_spaces();
                let right = self.parse_comparison_expression()?;
                left = Expression::BinaryOp(Box::new(left), BinaryOperator::And, Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_comparison_expression(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_single_expression()?;

        self.skip_spaces();

        // Check for comparison operators
        let op = if self.peek_two_chars() == Some(('=', '=')) {
            self.advance();
            self.advance();
            Some(BinaryOperator::Equal)
        } else if self.peek_two_chars() == Some(('!', '=')) {
            self.advance();
            self.advance();
            Some(BinaryOperator::NotEqual)
        } else if self.peek_two_chars() == Some(('<', '=')) {
            self.advance();
            self.advance();
            Some(BinaryOperator::LessEqual)
        } else if self.peek_two_chars() == Some(('>', '=')) {
            self.advance();
            self.advance();
            Some(BinaryOperator::GreaterEqual)
        } else if self.peek_char() == Some('<') {
            self.advance();
            Some(BinaryOperator::LessThan)
        } else if self.peek_char() == Some('>') {
            self.advance();
            Some(BinaryOperator::GreaterThan)
        } else {
            None
        };

        if let Some(operator) = op {
            self.skip_spaces();
            let right = self.parse_single_expression()?;
            Ok(Expression::BinaryOp(
                Box::new(left),
                operator,
                Box::new(right),
            ))
        } else {
            Ok(left)
        }
    }

    fn peek_two_chars(&self) -> Option<(char, char)> {
        if self.pos + 1 < self.input.len() {
            Some((self.input[self.pos], self.input[self.pos + 1]))
        } else {
            None
        }
    }

    fn parse_single_expression(&mut self) -> Result<Expression, ParseError> {
        self.skip_spaces();

        // Check for string literal
        if self.peek_char() == Some('"') {
            return self.parse_string_literal();
        }

        // Parse identifier (or numeric literal)
        let ident = self.parse_identifier()?;

        // Check if it's a numeric literal
        let mut expr = if ident.chars().all(|c| c.is_ascii_digit()) {
            // It's a number literal
            Expression::Literal(ident)
        } else if ident.starts_with('-')
            && ident.len() > 1
            && ident[1..].chars().all(|c| c.is_ascii_digit())
        {
            // Negative number literal
            Expression::Literal(ident)
        } else {
            // It's an identifier
            Expression::Simple(Identifier::new(ident))
        };

        // Parse accessors (property, dynamic, method)
        loop {
            self.skip_spaces();
            match self.peek_char() {
                Some('.') => {
                    self.consume_char('.');
                    let name = self.parse_identifier()?;

                    // Check if this is a method call
                    if self.peek_char() == Some('(') {
                        let method = self.parse_method_call(name)?;
                        expr = Expression::Method(Box::new(expr), method);
                    } else {
                        expr = Expression::Property(Box::new(expr), Identifier::new(name));
                    }
                }
                Some('[') => {
                    self.consume_char('[');
                    let index_expr = self.parse_expression()?;
                    if self.peek_char() != Some(']') {
                        return Err(ParseError::UnterminatedReference(self.line));
                    }
                    self.consume_char(']');
                    expr = Expression::Dynamic(Box::new(expr), Box::new(index_expr));
                }
                Some('=') if matches!(expr, Expression::Simple(_)) => {
                    // Assignment
                    self.consume_char('=');
                    self.skip_spaces();
                    if let Expression::Simple(ident) = expr {
                        let value = self.parse_single_expression()?;
                        return Ok(Expression::Assignment(ident, Box::new(value)));
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_method_call(&mut self, name: String) -> Result<MethodCall, ParseError> {
        self.consume_char('(');
        let mut args = Vec::new();

        self.skip_spaces();
        if self.peek_char() != Some(')') {
            loop {
                args.push(self.parse_expression()?);
                self.skip_spaces();

                if self.peek_char() == Some(',') {
                    self.consume_char(',');
                    self.skip_spaces();
                } else {
                    break;
                }
            }
        }

        if self.peek_char() != Some(')') {
            return Err(ParseError::InvalidSyntax(
                "Unterminated method call".to_string(),
                self.line,
            ));
        }
        self.consume_char(')');

        Ok(MethodCall::new(name).with_args(args))
    }

    fn parse_string_literal(&mut self) -> Result<Expression, ParseError> {
        let start_line = self.line;
        self.consume_char('"');
        let mut s = String::new();

        while let Some(&ch) = self.peek_char_ref() {
            if ch == '"' {
                self.consume_char('"');
                return Ok(Expression::Literal(s));
            } else if ch == '\\' {
                self.consume_char('\\');
                if let Some(&escaped) = self.peek_char_ref() {
                    s.push(escaped);
                    self.advance();
                }
            } else {
                s.push(ch);
                self.advance();
            }
        }

        Err(ParseError::UnterminatedString(start_line))
    }

    fn parse_inline(&mut self) -> Result<InlineList, ParseError> {
        let start_line = self.line;
        self.consume_char('{');

        // Check for special inline functions: {a} and {s}
        if self.peek_char() == Some('a') {
            let next_pos = self.pos + 1;
            if next_pos < self.input.len() && self.input[next_pos] == '}' {
                self.consume_char('a');
                self.consume_char('}');
                // Return a special inline that's handled differently in evaluator
                return Ok(InlineList::new(vec![InlineChoice::new(vec![
                    ContentPart::Article,
                ])]));
            }
        }

        if self.peek_char() == Some('s') {
            let next_pos = self.pos + 1;
            if next_pos < self.input.len() && self.input[next_pos] == '}' {
                self.consume_char('s');
                self.consume_char('}');
                return Ok(InlineList::new(vec![InlineChoice::new(vec![
                    ContentPart::Pluralize,
                ])]));
            }
        }

        // Check for number range: {n-m}
        if self.is_number_range() {
            return self.parse_inline_number_range();
        }

        // Check for letter range: {a-z}
        if self.is_letter_range() {
            return self.parse_inline_letter_range();
        }

        // Parse choices separated by |
        let mut choices = Vec::new();
        let mut content_buffer = Vec::new();

        while let Some(&ch) = self.peek_char_ref() {
            match ch {
                '}' => {
                    if !content_buffer.is_empty() || choices.is_empty() {
                        choices.push(InlineChoice::new(content_buffer.clone()));
                    }
                    self.consume_char('}');
                    return Ok(InlineList::new(choices));
                }
                '|' => {
                    choices.push(InlineChoice::new(content_buffer.clone()));
                    content_buffer = Vec::new();
                    self.consume_char('|');
                }
                '\\' => {
                    self.consume_char('\\');
                    let escaped = self.parse_escape()?;
                    content_buffer.push(ContentPart::Escape(escaped));
                }
                '[' => {
                    let expr = self.parse_reference()?;
                    content_buffer.push(ContentPart::Reference(expr));
                }
                '{' => {
                    let inline = self.parse_inline()?;
                    content_buffer.push(ContentPart::Inline(inline));
                }
                '^' => {
                    // Weight for this choice
                    self.consume_char('^');
                    let weight = self.parse_number()?;
                    let mut choice = InlineChoice::new(content_buffer.clone());
                    choice = choice.with_weight(weight);
                    choices.push(choice);
                    content_buffer = Vec::new();

                    // Expect | or }
                    if self.peek_char() == Some('|') {
                        self.consume_char('|');
                    }
                }
                _ => {
                    content_buffer.push(ContentPart::Text(ch.to_string()));
                    self.advance();
                }
            }
        }

        Err(ParseError::UnterminatedInline(start_line))
    }

    fn is_number_range(&self) -> bool {
        let mut i = self.pos;
        // Skip optional minus
        if i < self.input.len() && self.input[i] == '-' {
            i += 1;
        }
        // Expect digits
        let start = i;
        while i < self.input.len() && self.input[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
        // Expect '-'
        if i >= self.input.len() || self.input[i] != '-' {
            return false;
        }
        i += 1;
        // Skip optional minus
        if i < self.input.len() && self.input[i] == '-' {
            i += 1;
        }
        // Expect digits
        let start2 = i;
        while i < self.input.len() && self.input[i].is_ascii_digit() {
            i += 1;
        }
        if i == start2 {
            return false;
        }
        // Expect '}'
        i < self.input.len() && self.input[i] == '}'
    }

    fn is_letter_range(&self) -> bool {
        if self.pos + 3 >= self.input.len() {
            return false;
        }
        let c1 = self.input[self.pos];
        let c2 = self.input[self.pos + 1];
        let c3 = self.input[self.pos + 2];
        let c4 = self.input[self.pos + 3];

        c1.is_ascii_alphabetic() && c2 == '-' && c3.is_ascii_alphabetic() && c4 == '}'
    }

    fn parse_inline_number_range(&mut self) -> Result<InlineList, ParseError> {
        let start_num = self.parse_signed_integer()?;
        self.consume_char('-');
        let end_num = self.parse_signed_integer()?;
        self.consume_char('}');

        // Represent as a special inline with NumberRange expression
        let expr = Expression::NumberRange(start_num, end_num);
        Ok(InlineList::new(vec![InlineChoice::new(vec![
            ContentPart::Reference(expr),
        ])]))
    }

    fn parse_inline_letter_range(&mut self) -> Result<InlineList, ParseError> {
        let start_char = self.current_char().unwrap();
        self.advance();
        self.consume_char('-');
        let end_char = self.current_char().unwrap();
        self.advance();
        self.consume_char('}');

        // Represent as a special inline with LetterRange expression
        let expr = Expression::LetterRange(start_char, end_char);
        Ok(InlineList::new(vec![InlineChoice::new(vec![
            ContentPart::Reference(expr),
        ])]))
    }

    fn parse_escape(&mut self) -> Result<char, ParseError> {
        if let Some(&ch) = self.peek_char_ref() {
            self.advance();
            match ch {
                's' => Ok(' '), // \s = space
                't' => Ok('\t'),
                'n' => Ok('\n'),
                'r' => Ok('\r'),
                '\\' => Ok('\\'),
                '[' => Ok('['),
                ']' => Ok(']'),
                '{' => Ok('{'),
                '}' => Ok('}'),
                '=' => Ok('='),
                '^' => Ok('^'),
                '|' => Ok('|'),
                _ => Err(ParseError::InvalidEscape(ch, self.line)),
            }
        } else {
            Err(ParseError::UnexpectedEof)
        }
    }

    fn peek_identifier(&self) -> String {
        let mut ident = String::new();
        let mut pos = self.pos;

        // First character must be letter, underscore, or $
        if pos < self.input.len() {
            let ch = self.input[pos];
            if ch.is_ascii_alphabetic() || ch == '_' || ch == '$' {
                ident.push(ch);
                pos += 1;
            }
        }

        // Subsequent characters can be alphanumeric or underscore
        while pos < self.input.len() {
            let ch = self.input[pos];
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                pos += 1;
            } else {
                break;
            }
        }

        ident
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let mut ident = String::new();

        // First character must be letter, underscore, or $
        if let Some(&ch) = self.peek_char_ref() {
            if ch.is_ascii_alphabetic() || ch == '_' || ch == '$' {
                ident.push(ch);
                self.advance();
            }
        }

        // Subsequent characters can be alphanumeric or underscore
        while let Some(&ch) = self.peek_char_ref() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(ident)
    }

    fn parse_number(&mut self) -> Result<f64, ParseError> {
        let mut num_str = String::new();

        while let Some(&ch) = self.peek_char_ref() {
            if ch.is_ascii_digit() || ch == '.' {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        num_str
            .parse()
            .map_err(|_| ParseError::InvalidNumberRange(self.line))
    }

    fn parse_signed_integer(&mut self) -> Result<i64, ParseError> {
        let mut num_str = String::new();

        // Handle optional negative sign
        if self.peek_char() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        while let Some(&ch) = self.peek_char_ref() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        num_str
            .parse()
            .map_err(|_| ParseError::InvalidNumberRange(self.line))
    }

    fn detect_space_indent_unit(&mut self) {
        // Detect the space indentation unit from the first indented line
        if self.space_indent_unit.is_some() {
            return; // Already detected
        }

        let mut i = self.pos;
        let mut space_count = 0;

        // Count leading spaces
        while i < self.input.len() && self.input[i] == ' ' {
            space_count += 1;
            i += 1;
        }

        // If we found spaces, determine the unit
        if space_count >= 4 {
            self.space_indent_unit = Some(4);
        } else if space_count >= 2 {
            self.space_indent_unit = Some(2);
        }
    }

    fn get_indentation_level(&mut self) -> usize {
        let mut level = 0;
        let mut i = self.pos;

        while i < self.input.len() {
            match self.input[i] {
                '\t' => {
                    level += 1;
                    i += 1;
                }
                ' ' => {
                    // Detect space indent unit if not already done
                    if self.space_indent_unit.is_none() {
                        self.detect_space_indent_unit();
                    }

                    let unit = self.space_indent_unit.unwrap_or(2);

                    // Check if we have enough spaces for one indentation level
                    let mut has_full_unit = true;
                    for offset in 0..unit {
                        if i + offset >= self.input.len() || self.input[i + offset] != ' ' {
                            has_full_unit = false;
                            break;
                        }
                    }

                    if has_full_unit {
                        level += 1;
                        i += unit;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        level
    }

    fn skip_indent(&mut self, level: usize) {
        let unit = self.space_indent_unit.unwrap_or(2);

        for _ in 0..level {
            if self.peek_char() == Some('\t') {
                self.advance();
            } else if self.peek_char() == Some(' ') {
                // Skip the detected number of spaces
                for _ in 0..unit {
                    if self.peek_char() == Some(' ') {
                        self.advance();
                    }
                }
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), ParseError> {
        loop {
            self.skip_spaces();

            if self.peek_char() == Some('/') && self.peek_ahead(1) == Some('/') {
                self.skip_to_newline();
                if self.peek_char() == Some('\n') {
                    self.advance();
                }
            } else if self.peek_char() == Some('\n') || self.peek_char() == Some('\r') {
                self.advance();
            } else {
                break;
            }
        }
        Ok(())
    }

    fn skip_empty_lines(&mut self) {
        while !self.is_eof() {
            let start_pos = self.pos;
            let start_line = self.line;

            // Skip whitespace on this line
            while let Some(&ch) = self.peek_char_ref() {
                if ch == ' ' || ch == '\t' {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for comment
            if self.peek_char() == Some('/') && self.peek_ahead(1) == Some('/') {
                self.skip_to_newline();
            }

            // If we hit a newline, consume it and continue
            if self.peek_char() == Some('\n') || self.peek_char() == Some('\r') {
                self.advance();
            } else {
                // Not an empty line, restore position
                self.pos = start_pos;
                self.line = start_line;
                break;
            }
        }
    }

    fn skip_spaces(&mut self) {
        while let Some(&ch) = self.peek_char_ref() {
            if ch == ' ' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_to_newline(&mut self) {
        while let Some(&ch) = self.peek_char_ref() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            self.advance();
        }
    }

    fn consume_char(&mut self, expected: char) {
        if self.peek_char() == Some(expected) {
            self.advance();
        }
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input[self.pos] == '\n' {
                self.line += 1;
            }
            self.pos += 1;
        }
    }

    fn peek_char(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn peek_char_ref(&self) -> Option<&char> {
        self.input.get(self.pos)
    }

    fn current_char(&self) -> Option<char> {
        self.peek_char()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        if self.pos + offset < self.input.len() {
            Some(self.input[self.pos + offset])
        } else {
            None
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

pub fn parse(input: &str) -> Result<Program, ParseError> {
    let mut parser = Parser::new(input);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_list() {
        let input = "animal\n\tdog\n\tcat\n";
        let result = parse(input);
        let program = result.unwrap();
        assert_eq!(program.lists.len(), 1);
        assert_eq!(program.lists[0].name, "animal");
        assert_eq!(program.lists[0].items.len(), 2);
    }

    #[test]
    fn test_with_output() {
        let input = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a [animal].\n";
        let result = parse(input);
        let program = result.unwrap();
        assert_eq!(program.lists.len(), 2);
    }

    #[test]
    fn test_inline_list() {
        let input = "output\n\t{big|small} animal\n";
        assert_eq!(
            parse(input).unwrap(),
            Program {
                lists: [List {
                    name: "output".into(),
                    items: [Item {
                        content: [
                            ContentPart::Inline(InlineList {
                                choices: [
                                    InlineChoice {
                                        content: [
                                            ContentPart::Text("b".into()),
                                            ContentPart::Text("i".into()),
                                            ContentPart::Text("g".into())
                                        ]
                                        .into(),
                                        weight: None
                                    },
                                    InlineChoice {
                                        content: [
                                            ContentPart::Text("s".into()),
                                            ContentPart::Text("m".into()),
                                            ContentPart::Text("a".into()),
                                            ContentPart::Text("l".into()),
                                            ContentPart::Text("l".into())
                                        ]
                                        .into(),
                                        weight: None
                                    }
                                ]
                                .into()
                            }),
                            ContentPart::Text(" animal".into())
                        ]
                        .into(),
                        weight: None,
                        sublists: [].into()
                    }]
                    .into(),
                    output: None
                }]
                .into()
            }
        );
    }

    #[test]
    fn test_number_range() {
        let input = "output\n\tRolled {1-6}\n";
        assert_eq!(
            parse(input).unwrap(),
            Program {
                lists: [List {
                    name: "output".into(),
                    items: [Item {
                        content: [
                            ContentPart::Text("Rolled ".into()),
                            ContentPart::Inline(InlineList {
                                choices: [InlineChoice {
                                    content: [ContentPart::Reference(Expression::NumberRange(
                                        1, 6
                                    ))]
                                    .into(),
                                    weight: None
                                }]
                                .into()
                            })
                        ]
                        .into(),
                        weight: None,
                        sublists: [].into()
                    }]
                    .into(),
                    output: None
                }]
                .into()
            }
        );
    }

    #[test]
    fn test_weights() {
        let input = "animal\n\tdog^2\n\tcat^0.5\n\tbird\n";
        let result = parse(input);
        let program = result.unwrap();
        assert_eq!(program.lists[0].items[0].weight, Some(2.0));
        assert_eq!(program.lists[0].items[1].weight, Some(0.5));
        assert_eq!(program.lists[0].items[2].weight, None);
    }
}
