/// Parser for Perchance language
use crate::ast::*;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEof,
    InvalidIndentation { span: Span },
    InvalidSyntax { message: String, span: Span },
    UnterminatedReference { span: Span },
    UnterminatedInline { span: Span },
    UnterminatedString { span: Span },
    InvalidEscape { ch: char, span: Span },
    InvalidNumberRange { span: Span },
    EmptyListName { span: Span },
}

impl ParseError {
    pub fn span(&self) -> Option<Span> {
        match self {
            ParseError::UnexpectedEof => None,
            ParseError::InvalidIndentation { span } => Some(*span),
            ParseError::InvalidSyntax { span, .. } => Some(*span),
            ParseError::UnterminatedReference { span } => Some(*span),
            ParseError::UnterminatedInline { span } => Some(*span),
            ParseError::UnterminatedString { span } => Some(*span),
            ParseError::InvalidEscape { span, .. } => Some(*span),
            ParseError::InvalidNumberRange { span } => Some(*span),
            ParseError::EmptyListName { span } => Some(*span),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "Unexpected end of file"),
            ParseError::InvalidIndentation { span } => {
                write!(f, "Invalid indentation at position {}", span.start)
            }
            ParseError::InvalidSyntax { message, span } => {
                write!(f, "{} at position {}", message, span.start)
            }
            ParseError::UnterminatedReference { span } => {
                write!(f, "Unterminated reference at position {}", span.start)
            }
            ParseError::UnterminatedInline { span } => {
                write!(f, "Unterminated inline list at position {}", span.start)
            }
            ParseError::UnterminatedString { span } => {
                write!(f, "Unterminated string at position {}", span.start)
            }
            ParseError::InvalidEscape { ch, span } => {
                write!(
                    f,
                    "Invalid escape sequence '\\{}' at position {}",
                    ch, span.start
                )
            }
            ParseError::InvalidNumberRange { span } => {
                write!(f, "Invalid number range at position {}", span.start)
            }
            ParseError::EmptyListName { span } => {
                write!(f, "Empty list name at position {}", span.start)
            }
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

    /// Get current position (for span tracking)
    fn current_pos(&self) -> usize {
        self.pos
    }

    /// Create a span from start to current position
    fn span_from(&self, start: usize) -> Span {
        Span::new(start, self.pos)
    }

    /// Create a span for a range
    fn make_span(&self, start: usize, end: usize) -> Span {
        Span::new(start, end)
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let start = self.current_pos();
        let mut program = Program::new();

        while !self.is_eof() {
            self.skip_whitespace_and_comments()?;
            if self.is_eof() {
                break;
            }

            // Only parse lists at the top level (no indentation)
            if self.get_indentation_level() == 0 {
                // Reset indentation detection for each top-level list to support mixed indentation
                self.space_indent_unit = None;
                let list = self.parse_list(0)?;
                program.add_list(list);
            } else {
                let span = self.make_span(self.pos, self.pos + 1);
                return Err(ParseError::InvalidIndentation { span });
            }
        }

        program.span = self.span_from(start);
        Ok(program)
    }

    fn parse_list(&mut self, expected_indent: usize) -> Result<List, ParseError> {
        let start = self.current_pos();

        // Parse list name
        let name_ident = self.parse_identifier()?;
        if name_ident.name.is_empty() {
            let span = self.span_from(start);
            return Err(ParseError::EmptyListName { span });
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
            let span = self.span_from(start);
            let mut list = List::new_with_span(name_ident.name, span);
            list.set_output(output_content);
            return Ok(list);
        }

        self.skip_to_newline();
        self.consume_char('\n');

        let mut list = List::new(name_ident.name);
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
                        let span = self.make_span(self.pos, self.pos + 1);
                        return Err(ParseError::InvalidSyntax {
                            message: "Expected '=' after $output".to_string(),
                            span,
                        });
                    }
                } else {
                    let item = self.parse_item(item_indent)?;
                    list.add_item(item);
                }
            } else {
                // Too much indentation
                let span = self.make_span(self.pos, self.pos + 1);
                return Err(ParseError::InvalidIndentation { span });
            }
        }

        list.span = self.span_from(start);
        Ok(list)
    }

    fn parse_item(&mut self, expected_indent: usize) -> Result<Item, ParseError> {
        let start = self.current_pos();

        // Parse item content until newline or weight
        let content = self.parse_content_until_newline()?;

        // Check for weight (^number or ^[expression])
        let weight = if self.peek_char() == Some('^') {
            self.consume_char('^');
            if self.peek_char() == Some('[') {
                // Dynamic weight: ^[expression]
                self.consume_char('[');
                let expr = self.parse_expression_in_reference()?;
                if self.peek_char() != Some(']') {
                    let span = self.make_span(self.pos, self.pos + 1);
                    return Err(ParseError::UnterminatedReference { span });
                }
                self.consume_char(']');
                Some(ItemWeight::Dynamic(Box::new(expr)))
            } else {
                // Static weight: ^number
                Some(ItemWeight::Static(self.parse_number()?))
            }
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
            if let ContentPart::Text(ref s, _) = content[0] {
                // Trim whitespace from the identifier
                let trimmed = s.trim();
                // Check if it's a valid identifier (letters, numbers, underscore only)
                if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') && !trimmed.is_empty() {
                    Some(trimmed.to_string())
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
        // If we have a simple name and indented content, treat all indented items as a single sublist
        if let Some(sublist_name) = simple_name {
            // Check if there are indented items
            self.skip_empty_lines();
            if !self.is_eof() && self.get_indentation_level() == expected_indent + 1 {
                let sublist_indent = expected_indent + 1;
                let sublist_start = self.current_pos();

                // Create a single sublist with the parent's name
                let mut sublist = List::new(sublist_name);

                // Parse all indented items as items in this sublist
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

                        // Check if this is a property assignment (name = value)
                        let prop_name_str = self.peek_identifier();
                        if !prop_name_str.is_empty() {
                            let saved_pos = self.pos;
                            let _ident = self.parse_identifier();
                            self.skip_spaces();

                            if self.peek_char() == Some('=')
                                && self.peek_two_chars() != Some(('=', '='))
                            {
                                // This is a property assignment
                                self.consume_char('=');
                                self.skip_spaces();

                                let prop_list_start = saved_pos;
                                // Parse the value
                                let value_content = self.parse_content_until_newline()?;
                                self.skip_to_newline();
                                if !self.is_eof() {
                                    self.consume_char('\n');
                                }

                                // Create a sublist for this property
                                let prop_list_span = self.span_from(prop_list_start);
                                let mut prop_list =
                                    List::new_with_span(prop_name_str.clone(), prop_list_span);
                                let prop_item = Item::new(value_content);
                                prop_list.add_item(prop_item);

                                // Add this as a subitem with the property as a sublist
                                let mut prop_subitem = Item::new(vec![]);
                                prop_subitem.add_sublist(prop_list);
                                sublist.add_item(prop_subitem);
                                continue;
                            } else {
                                // Not a property assignment, restore position and parse normally
                                self.pos = saved_pos;
                            }
                        }

                        let subitem = self.parse_item(sublist_indent)?;
                        sublist.add_item(subitem);
                    } else {
                        // Deeper nesting belongs to the subitem
                        break;
                    }
                }

                // Clear the content and add the single sublist
                item.content.clear();
                sublist.span = self.span_from(sublist_start);
                item.add_sublist(sublist);
            }
        }

        item.span = self.span_from(start);
        Ok(item)
    }

    pub fn parse_content_until_newline(&mut self) -> Result<Vec<ContentPart>, ParseError> {
        let mut parts = Vec::new();
        let mut text_buffer = String::new();
        let mut text_start = self.current_pos();

        while let Some(&ch) = self.peek_char_ref() {
            match ch {
                '\n' | '\r' => break,
                '/' if self.peek_ahead(1) == Some('/') => break, // Comment
                '^' => break,                                    // Weight marker
                '\\' => {
                    // Escape sequence
                    if !text_buffer.is_empty() {
                        let span = self.make_span(text_start, self.current_pos());
                        parts.push(ContentPart::Text(text_buffer.clone(), span));
                        text_buffer.clear();
                    }
                    let escape_start = self.current_pos();
                    self.consume_char('\\');
                    let escaped = self.parse_escape()?;
                    let span = self.span_from(escape_start);
                    parts.push(ContentPart::Escape(escaped, span));
                    text_start = self.current_pos();
                }
                '[' => {
                    // Reference
                    if !text_buffer.is_empty() {
                        let span = self.make_span(text_start, self.current_pos());
                        parts.push(ContentPart::Text(text_buffer.clone(), span));
                        text_buffer.clear();
                    }
                    let ref_start = self.current_pos();
                    let expr = self.parse_reference()?;
                    let span = self.span_from(ref_start);
                    parts.push(ContentPart::Reference(expr, span));
                    text_start = self.current_pos();
                }
                '{' => {
                    // Inline list or number range
                    if !text_buffer.is_empty() {
                        let span = self.make_span(text_start, self.current_pos());
                        parts.push(ContentPart::Text(text_buffer.clone(), span));
                        text_buffer.clear();
                    }
                    let inline_start = self.current_pos();
                    let inline = self.parse_inline()?;
                    let span = self.span_from(inline_start);
                    parts.push(ContentPart::Inline(inline, span));
                    text_start = self.current_pos();
                }
                _ => {
                    if text_buffer.is_empty() {
                        text_start = self.current_pos();
                    }
                    text_buffer.push(ch);
                    self.advance();
                }
            }
        }

        if !text_buffer.is_empty() {
            let span = self.make_span(text_start, self.current_pos());
            parts.push(ContentPart::Text(text_buffer, span));
        }

        Ok(parts)
    }

    fn parse_reference(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        self.consume_char('[');

        let expr = self.parse_expression()?;

        if self.peek_char() != Some(']') {
            let span = self.span_from(start);
            return Err(ParseError::UnterminatedReference { span });
        }
        self.consume_char(']');

        Ok(expr)
    }

    fn parse_expression_in_reference(&mut self) -> Result<Expression, ParseError> {
        // Similar to parse_reference but assumes '[' is already consumed
        self.parse_expression()
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
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
            let span = self.span_from(start);
            Ok(Expression::Sequence(exprs, output.map(Box::new), span))
        } else {
            Ok(first)
        }
    }

    fn parse_ternary_expression(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        // Parse ternary conditional: condition ? true_expr : false_expr
        let first = self.parse_or_expression()?;

        self.skip_spaces();
        if self.peek_char() == Some('?') {
            self.consume_char('?');
            self.skip_spaces();
            let true_expr = self.parse_or_expression()?;
            self.skip_spaces();

            if self.peek_char() != Some(':') {
                let span = self.make_span(self.pos, self.pos + 1);
                return Err(ParseError::InvalidSyntax {
                    message: "Expected ':' in ternary expression".to_string(),
                    span,
                });
            }
            self.consume_char(':');
            self.skip_spaces();

            let false_expr = self.parse_ternary_expression()?;
            let span = self.span_from(start);
            return Ok(Expression::Conditional(
                Box::new(first),
                Box::new(true_expr),
                Box::new(false_expr),
                span,
            ));
        }

        Ok(first)
    }

    fn parse_or_expression(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        let mut left = self.parse_and_expression()?;

        loop {
            self.skip_spaces();
            if self.peek_two_chars() == Some(('|', '|')) {
                self.advance();
                self.advance();
                self.skip_spaces();

                // Check if left is a Property expression - if so, this is property fallback
                if let Expression::Property(base, prop, _) = left {
                    let fallback = self.parse_and_expression()?;
                    let span = self.span_from(start);
                    left = Expression::PropertyWithFallback(base, prop, Box::new(fallback), span);
                } else {
                    // Otherwise, it's a logical OR
                    let right = self.parse_and_expression()?;
                    let span = self.span_from(start);
                    left = Expression::BinaryOp(
                        Box::new(left),
                        BinaryOperator::Or,
                        Box::new(right),
                        span,
                    );
                }
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        let mut left = self.parse_comparison_expression()?;

        loop {
            self.skip_spaces();
            if self.peek_two_chars() == Some(('&', '&')) {
                self.advance();
                self.advance();
                self.skip_spaces();
                let right = self.parse_comparison_expression()?;
                let span = self.span_from(start);
                left = Expression::BinaryOp(
                    Box::new(left),
                    BinaryOperator::And,
                    Box::new(right),
                    span,
                );
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_comparison_expression(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        let left = self.parse_additive_expression()?;

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
            let right = self.parse_additive_expression()?;
            let span = self.span_from(start);
            Ok(Expression::BinaryOp(
                Box::new(left),
                operator,
                Box::new(right),
                span,
            ))
        } else {
            Ok(left)
        }
    }

    fn parse_additive_expression(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        let mut left = self.parse_multiplicative_expression()?;

        loop {
            self.skip_spaces();
            let op = match self.peek_char() {
                Some('+') => {
                    self.advance();
                    Some(BinaryOperator::Add)
                }
                Some('-') if self.peek_two_chars() != Some(('-', '>')) => {
                    // Make sure it's not part of a range like {1-10}
                    // Also check it's not a negative number at start of expression
                    self.advance();
                    Some(BinaryOperator::Subtract)
                }
                _ => None,
            };

            if let Some(operator) = op {
                self.skip_spaces();
                let right = self.parse_multiplicative_expression()?;
                let span = self.span_from(start);
                left = Expression::BinaryOp(Box::new(left), operator, Box::new(right), span);
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_multiplicative_expression(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        let mut left = self.parse_single_expression()?;

        loop {
            self.skip_spaces();
            let op = match self.peek_char() {
                Some('*') => {
                    self.advance();
                    Some(BinaryOperator::Multiply)
                }
                Some('/') => {
                    self.advance();
                    Some(BinaryOperator::Divide)
                }
                Some('%') => {
                    self.advance();
                    Some(BinaryOperator::Modulo)
                }
                _ => None,
            };

            if let Some(operator) = op {
                self.skip_spaces();
                let right = self.parse_single_expression()?;
                let span = self.span_from(start);
                left = Expression::BinaryOp(Box::new(left), operator, Box::new(right), span);
            } else {
                break;
            }
        }

        Ok(left)
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
        let start = self.current_pos();

        // Check for string literal
        if self.peek_char() == Some('"') {
            return self.parse_string_literal();
        }

        // Check for numeric literal (starts with digit or negative sign followed by digit)
        if let Some(&ch) = self.peek_char_ref() {
            if ch.is_ascii_digit()
                || (ch == '-'
                    && self
                        .peek_two_chars()
                        .is_some_and(|(_, c2)| c2.is_ascii_digit()))
            {
                // Parse as number
                let num = self.parse_number()?;
                let span = self.span_from(start);
                let mut expr = Expression::Number(num, span);

                // Parse accessors (property, dynamic, method) if any
                loop {
                    self.skip_spaces();
                    match self.peek_char() {
                        Some('.') => {
                            self.consume_char('.');
                            let prop_ident = self.parse_identifier()?;

                            // Check if this is a method call
                            if self.peek_char() == Some('(') {
                                let method = self.parse_method_call(prop_ident.name.clone())?;
                                let span = self.span_from(start);
                                expr = Expression::Method(Box::new(expr), method, span);
                            } else {
                                let span = self.span_from(start);
                                expr = Expression::Property(Box::new(expr), prop_ident, span);
                            }
                        }
                        _ => break,
                    }
                }

                return Ok(expr);
            }
        }

        // Parse identifier
        let ident = self.parse_identifier()?;
        let span = self.span_from(start);
        let mut expr = Expression::Simple(ident.clone(), span);

        // Parse accessors (property, dynamic, method)
        loop {
            self.skip_spaces();
            match self.peek_char() {
                Some('.') => {
                    self.consume_char('.');
                    let prop_ident = self.parse_identifier()?;

                    // Check if this is a method call
                    if self.peek_char() == Some('(') {
                        let method = self.parse_method_call(prop_ident.name.clone())?;
                        let span = self.span_from(start);
                        expr = Expression::Method(Box::new(expr), method, span);
                    } else {
                        let span = self.span_from(start);
                        expr = Expression::Property(Box::new(expr), prop_ident, span);
                    }
                }
                Some('[') => {
                    let bracket_start = self.current_pos();
                    self.consume_char('[');
                    let index_expr = self.parse_expression()?;
                    if self.peek_char() != Some(']') {
                        let span = self.span_from(bracket_start);
                        return Err(ParseError::UnterminatedReference { span });
                    }
                    self.consume_char(']');
                    let span = self.span_from(start);
                    expr = Expression::Dynamic(Box::new(expr), Box::new(index_expr), span);
                }
                Some('(') if matches!(expr, Expression::Simple(_, _)) => {
                    // Direct function call like joinLists(arg1, arg2)
                    if let Expression::Simple(ref ident, _) = expr {
                        let method = self.parse_method_call(ident.name.clone())?;
                        let span = self.span_from(start);
                        expr = Expression::Method(Box::new(expr), method, span);
                    }
                }
                Some('=') if self.peek_two_chars() != Some(('=', '=')) => {
                    // Assignment or property assignment (but not ==)
                    self.consume_char('=');
                    self.skip_spaces();

                    match expr {
                        Expression::Simple(ident, _) => {
                            // Simple assignment: [x = value]
                            // Parse full expression to allow math operations, etc.
                            let value = self.parse_ternary_expression()?;
                            let span = self.span_from(start);
                            return Ok(Expression::Assignment(ident, Box::new(value), span));
                        }
                        Expression::Property(base, prop, _) => {
                            // Property assignment: [this.property = value]
                            // Parse full expression to allow math operations, etc.
                            let value = self.parse_ternary_expression()?;
                            let span = self.span_from(start);
                            return Ok(Expression::PropertyAssignment(
                                base,
                                prop,
                                Box::new(value),
                                span,
                            ));
                        }
                        _ => {
                            let span = self.span_from(start);
                            return Err(ParseError::InvalidSyntax {
                                message: "Invalid left-hand side in assignment".to_string(),
                                span,
                            });
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_method_call(&mut self, name: String) -> Result<MethodCall, ParseError> {
        let start = self.current_pos();
        self.consume_char('(');
        let mut args = Vec::new();

        self.skip_spaces();
        if self.peek_char() != Some(')') {
            loop {
                // Use parse_ternary_expression instead of parse_expression
                // to avoid treating commas as sequence separators
                args.push(self.parse_ternary_expression()?);
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
            let span = self.span_from(start);
            return Err(ParseError::InvalidSyntax {
                message: "Unterminated method call".to_string(),
                span,
            });
        }
        self.consume_char(')');

        let span = self.span_from(start);
        Ok(MethodCall::new_with_span(name, span).with_args(args))
    }

    fn parse_string_literal(&mut self) -> Result<Expression, ParseError> {
        let start = self.current_pos();
        self.consume_char('"');
        let mut s = String::new();

        while let Some(&ch) = self.peek_char_ref() {
            if ch == '"' {
                self.consume_char('"');
                let span = self.span_from(start);
                return Ok(Expression::Literal(s, span));
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

        let span = self.span_from(start);
        Err(ParseError::UnterminatedString { span })
    }

    fn parse_inline(&mut self) -> Result<InlineList, ParseError> {
        let start = self.current_pos();
        self.consume_char('{');

        // Check for import: {import:generator-name}
        if self.peek_identifier().starts_with("import") {
            let saved_pos = self.pos;
            let ident = self.parse_identifier()?;
            if ident.name == "import" && self.peek_char() == Some(':') {
                self.consume_char(':');
                let import_start = self.current_pos();
                // Parse the generator name (everything until })
                let mut generator_name = String::new();
                while let Some(&ch) = self.peek_char_ref() {
                    if ch == '}' {
                        self.consume_char('}');
                        let import_span = self.span_from(import_start);
                        let ref_span = self.span_from(start);
                        let inline_span = self.span_from(start);
                        let choice_span = self.span_from(start);
                        // Return the import as an inline list with a single choice containing an Import expression
                        return Ok(InlineList::new_with_span(
                            vec![InlineChoice::new_with_span(
                                vec![ContentPart::Reference(
                                    Expression::Import(generator_name, import_span),
                                    ref_span,
                                )],
                                choice_span,
                            )],
                            inline_span,
                        ));
                    }
                    generator_name.push(ch);
                    self.advance();
                }
                let span = self.span_from(start);
                return Err(ParseError::UnterminatedInline { span });
            } else {
                // Not an import, restore position
                self.pos = saved_pos;
            }
        }

        // Check for special inline functions: {a} and {s}
        if self.peek_char() == Some('a') {
            let next_pos = self.pos + 1;
            if next_pos < self.input.len() && self.input[next_pos] == '}' {
                self.consume_char('a');
                self.consume_char('}');
                let article_span = self.span_from(start);
                let choice_span = self.span_from(start);
                let inline_span = self.span_from(start);
                // Return a special inline that's handled differently in evaluator
                return Ok(InlineList::new_with_span(
                    vec![InlineChoice::new_with_span(
                        vec![ContentPart::Article(article_span)],
                        choice_span,
                    )],
                    inline_span,
                ));
            }
        }

        if self.peek_char() == Some('s') {
            let next_pos = self.pos + 1;
            if next_pos < self.input.len() && self.input[next_pos] == '}' {
                self.consume_char('s');
                self.consume_char('}');
                let pluralize_span = self.span_from(start);
                let choice_span = self.span_from(start);
                let inline_span = self.span_from(start);
                return Ok(InlineList::new_with_span(
                    vec![InlineChoice::new_with_span(
                        vec![ContentPart::Pluralize(pluralize_span)],
                        choice_span,
                    )],
                    inline_span,
                ));
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
        let mut choice_start = self.current_pos();

        while let Some(&ch) = self.peek_char_ref() {
            match ch {
                '}' => {
                    if !content_buffer.is_empty() || choices.is_empty() {
                        let choice_span = self.span_from(choice_start);
                        choices.push(InlineChoice::new_with_span(
                            content_buffer.clone(),
                            choice_span,
                        ));
                    }
                    self.consume_char('}');
                    let inline_span = self.span_from(start);
                    return Ok(InlineList::new_with_span(choices, inline_span));
                }
                '|' => {
                    let choice_span = self.span_from(choice_start);
                    choices.push(InlineChoice::new_with_span(
                        content_buffer.clone(),
                        choice_span,
                    ));
                    content_buffer = Vec::new();
                    self.consume_char('|');
                    choice_start = self.current_pos();
                }
                '\\' => {
                    let escape_start = self.current_pos();
                    self.consume_char('\\');
                    let escaped = self.parse_escape()?;
                    let span = self.span_from(escape_start);
                    content_buffer.push(ContentPart::Escape(escaped, span));
                }
                '[' => {
                    let ref_start = self.current_pos();
                    let expr = self.parse_reference()?;
                    let span = self.span_from(ref_start);
                    content_buffer.push(ContentPart::Reference(expr, span));
                }
                '{' => {
                    let inline_start = self.current_pos();
                    let inline = self.parse_inline()?;
                    let span = self.span_from(inline_start);
                    content_buffer.push(ContentPart::Inline(inline, span));
                }
                '^' => {
                    // Weight for this choice (^number or ^[expression])
                    self.consume_char('^');
                    let weight = if self.peek_char() == Some('[') {
                        // Dynamic weight: ^[expression]
                        let bracket_start = self.current_pos();
                        self.consume_char('[');
                        let expr = self.parse_expression_in_reference()?;
                        if self.peek_char() != Some(']') {
                            let span = self.span_from(bracket_start);
                            return Err(ParseError::UnterminatedReference { span });
                        }
                        self.consume_char(']');
                        ItemWeight::Dynamic(Box::new(expr))
                    } else {
                        // Static weight: ^number
                        ItemWeight::Static(self.parse_number()?)
                    };
                    let choice_span = self.span_from(choice_start);
                    let mut choice =
                        InlineChoice::new_with_span(content_buffer.clone(), choice_span);
                    choice = choice.with_weight(weight);
                    choices.push(choice);
                    content_buffer = Vec::new();

                    // Expect | or }
                    if self.peek_char() == Some('|') {
                        self.consume_char('|');
                        choice_start = self.current_pos();
                    }
                }
                _ => {
                    let text_start = self.current_pos();
                    content_buffer.push(ContentPart::Text(
                        ch.to_string(),
                        self.make_span(text_start, text_start + 1),
                    ));
                    self.advance();
                }
            }
        }

        let span = self.span_from(start);
        Err(ParseError::UnterminatedInline { span })
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
        let start = self.current_pos();
        let start_num = self.parse_signed_integer()?;
        self.consume_char('-');
        let end_num = self.parse_signed_integer()?;
        self.consume_char('}');

        // Represent as a special inline with NumberRange expression
        let expr_span = self.span_from(start);
        let ref_span = self.span_from(start);
        let choice_span = self.span_from(start);
        let inline_span = self.span_from(start);
        let expr = Expression::NumberRange(start_num, end_num, expr_span);
        Ok(InlineList::new_with_span(
            vec![InlineChoice::new_with_span(
                vec![ContentPart::Reference(expr, ref_span)],
                choice_span,
            )],
            inline_span,
        ))
    }

    fn parse_inline_letter_range(&mut self) -> Result<InlineList, ParseError> {
        let start = self.current_pos();
        let start_char = self.current_char().unwrap();
        self.advance();
        self.consume_char('-');
        let end_char = self.current_char().unwrap();
        self.advance();
        self.consume_char('}');

        // Represent as a special inline with LetterRange expression
        let expr_span = self.span_from(start);
        let ref_span = self.span_from(start);
        let choice_span = self.span_from(start);
        let inline_span = self.span_from(start);
        let expr = Expression::LetterRange(start_char, end_char, expr_span);
        Ok(InlineList::new_with_span(
            vec![InlineChoice::new_with_span(
                vec![ContentPart::Reference(expr, ref_span)],
                choice_span,
            )],
            inline_span,
        ))
    }

    fn parse_escape(&mut self) -> Result<char, ParseError> {
        if let Some(&ch) = self.peek_char_ref() {
            let start = self.current_pos();
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
                _ => {
                    let span = self.make_span(start, self.pos);
                    Err(ParseError::InvalidEscape { ch, span })
                }
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

    fn parse_identifier(&mut self) -> Result<Identifier, ParseError> {
        let start = self.current_pos();
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

        let span = self.span_from(start);
        Ok(Identifier::new_with_span(ident, span))
    }

    fn parse_number(&mut self) -> Result<f64, ParseError> {
        let start = self.current_pos();
        let mut num_str = String::new();

        while let Some(&ch) = self.peek_char_ref() {
            if ch.is_ascii_digit() || ch == '.' {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        num_str.parse().map_err(|_| {
            let span = self.span_from(start);
            ParseError::InvalidNumberRange { span }
        })
    }

    fn parse_signed_integer(&mut self) -> Result<i64, ParseError> {
        let start = self.current_pos();
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

        num_str.parse().map_err(|_| {
            let span = self.span_from(start);
            ParseError::InvalidNumberRange { span }
        })
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
        // Breakdown of positions:
        // 0-5: "output"
        // 6: "\n"
        // 7: "\t"
        // 8-18: "{big|small}"
        // 19-25: " animal"
        // 26: "\n"

        assert_eq!(
            parse(input).unwrap(),
            Program {
                lists: vec![List {
                    name: "output".into(),
                    items: vec![Item {
                        content: vec![
                            ContentPart::Inline(
                                InlineList {
                                    choices: vec![
                                        InlineChoice {
                                            content: vec![
                                                ContentPart::Text("b".into(), Span::new(9, 10)),
                                                ContentPart::Text("i".into(), Span::new(10, 11)),
                                                ContentPart::Text("g".into(), Span::new(11, 12)),
                                            ],
                                            weight: None,
                                            span: Span::new(9, 12),
                                        },
                                        InlineChoice {
                                            content: vec![
                                                ContentPart::Text("s".into(), Span::new(13, 14)),
                                                ContentPart::Text("m".into(), Span::new(14, 15)),
                                                ContentPart::Text("a".into(), Span::new(15, 16)),
                                                ContentPart::Text("l".into(), Span::new(16, 17)),
                                                ContentPart::Text("l".into(), Span::new(17, 18)),
                                            ],
                                            weight: None,
                                            span: Span::new(13, 18),
                                        }
                                    ],
                                    span: Span::new(8, 19),
                                },
                                Span::new(8, 19),
                            ),
                            ContentPart::Text(" animal".into(), Span::new(19, 26))
                        ],
                        weight: None,
                        sublists: vec![],
                        span: Span::new(8, 27),
                    }],
                    output: None,
                    span: Span::new(0, 27),
                }],
                span: Span::new(0, 27),
            }
        );
    }

    #[test]
    fn test_number_range() {
        let input = "output\n\tRolled {1-6}\n";
        let program = parse(input).unwrap();
        assert_eq!(program.lists.len(), 1);
        assert_eq!(program.lists[0].name, "output");
        assert_eq!(program.lists[0].items.len(), 1);

        // Check the content structure without checking spans
        let content = &program.lists[0].items[0].content;
        assert_eq!(content.len(), 2);

        // First part should be text
        match &content[0] {
            ContentPart::Text(text, _) => {
                assert_eq!(text, "Rolled ");
            }
            _ => panic!("Expected Text"),
        }

        // Second part should be inline with number range
        match &content[1] {
            ContentPart::Inline(inline, _) => {
                assert_eq!(inline.choices.len(), 1);
                match &inline.choices[0].content[0] {
                    ContentPart::Reference(Expression::NumberRange(start, end, _), _) => {
                        assert_eq!(*start, 1);
                        assert_eq!(*end, 6);
                    }
                    _ => panic!("Expected NumberRange"),
                }
            }
            _ => panic!("Expected Inline"),
        }
    }

    #[test]
    fn test_weights() {
        let input = "animal\n\tdog^2\n\tcat^0.5\n\tbird\n";
        let result = parse(input);
        let program = result.unwrap();
        assert_eq!(
            program.lists[0].items[0].weight,
            Some(ItemWeight::Static(2.0))
        );
        assert_eq!(
            program.lists[0].items[1].weight,
            Some(ItemWeight::Static(0.5))
        );
        assert_eq!(program.lists[0].items[2].weight, None);
    }
}
