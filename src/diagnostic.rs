/// Diagnostic reporting using ariadne for beautiful error messages
use crate::compiler::CompileError;
use crate::evaluator::EvalError;
use crate::parser::ParseError;
use crate::span::Span;
use ariadne::{Color, Label, Report, ReportKind, Source};
use std::ops::Range;

/// Convert a span to a range for ariadne
fn span_to_range(span: Span) -> Range<usize> {
    span.range()
}

/// Report a parse error with beautiful formatting
pub fn report_parse_error(source_name: &str, source: &str, error: &ParseError) -> String {
    let mut output = Vec::new();

    let report = match error {
        ParseError::UnexpectedEof => {
            Report::build(ReportKind::Error, source_name, source.len().saturating_sub(1))
                .with_message("Unexpected end of file")
                .with_label(
                    Label::new((source_name, source.len().saturating_sub(1)..source.len()))
                        .with_message("unexpected end of file")
                        .with_color(Color::Red),
                )
                .with_note("The input ended unexpectedly while parsing")
                .finish()
        }
        ParseError::InvalidIndentation { span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Invalid indentation")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message("this indentation is not valid")
                        .with_color(Color::Red),
                )
                .with_note("Ensure consistent indentation (tabs or 2/4 spaces)")
                .finish()
        }
        ParseError::InvalidSyntax { message, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Syntax error: {}", message))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(message)
                        .with_color(Color::Red),
                )
                .finish()
        }
        ParseError::UnterminatedReference { span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Unterminated reference")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message("this reference is missing a closing ']'")
                        .with_color(Color::Red),
                )
                .with_help("Add a closing ']' to complete the reference")
                .finish()
        }
        ParseError::UnterminatedInline { span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Unterminated inline list")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message("this inline list is missing a closing '}'")
                        .with_color(Color::Red),
                )
                .with_help("Add a closing '}' to complete the inline list")
                .finish()
        }
        ParseError::UnterminatedString { span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Unterminated string")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message("this string is missing a closing '\"'")
                        .with_color(Color::Red),
                )
                .with_help("Add a closing '\"' to complete the string")
                .finish()
        }
        ParseError::InvalidEscape { ch, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Invalid escape sequence: \\{}", ch))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("'\\{}' is not a valid escape sequence", ch))
                        .with_color(Color::Red),
                )
                .with_note("Valid escape sequences: \\s (space), \\t, \\n, \\r, \\\\, \\[, \\], \\{, \\}, \\=, \\^, \\|")
                .finish()
        }
        ParseError::InvalidNumberRange { span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Invalid number range")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message("this number range is malformed")
                        .with_color(Color::Red),
                )
                .with_help("Number ranges should be in the format {start-end}, e.g., {1-10}")
                .finish()
        }
        ParseError::EmptyListName { span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Empty list name")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message("list names cannot be empty")
                        .with_color(Color::Red),
                )
                .with_help("Provide a valid identifier for the list name")
                .finish()
        }
    };

    report
        .write((source_name, Source::from(source)), &mut output)
        .expect("Failed to write diagnostic");

    String::from_utf8(output).expect("Invalid UTF-8 in diagnostic output")
}

/// Report a compile error with beautiful formatting
pub fn report_compile_error(source_name: &str, source: &str, error: &CompileError) -> String {
    let mut output = Vec::new();

    let report = match error {
        CompileError::UndefinedList { name, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Undefined list: '{}'", name))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("list '{}' is not defined", name))
                        .with_color(Color::Red),
                )
                .with_help(format!("Define the '{}' list before using it", name))
                .finish()
        }
        CompileError::EmptyList { name, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Empty list: '{}'", name))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("list '{}' has no items", name))
                        .with_color(Color::Red),
                )
                .with_help("Add at least one item to the list or set an $output property")
                .finish()
        }
        CompileError::DuplicateList { name, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Duplicate list: '{}'", name))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("list '{}' is already defined", name))
                        .with_color(Color::Red),
                )
                .with_help(format!("Rename this list or remove the duplicate definition of '{}'", name))
                .finish()
        }
        CompileError::InvalidWeight { message, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Invalid weight: {}", message))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(message)
                        .with_color(Color::Red),
                )
                .with_help("Weights must be non-negative numbers")
                .finish()
        }
    };

    report
        .write((source_name, Source::from(source)), &mut output)
        .expect("Failed to write diagnostic");

    String::from_utf8(output).expect("Invalid UTF-8 in diagnostic output")
}

/// Report an evaluation error with beautiful formatting
pub fn report_eval_error(source_name: &str, source: &str, error: &EvalError) -> String {
    let mut output = Vec::new();

    let report = match error {
        EvalError::UndefinedList { name, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Undefined list: '{}'", name))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("list '{}' is not defined", name))
                        .with_color(Color::Red),
                )
                .with_help(format!("Define the '{}' list before using it, or check for typos", name))
                .finish()
        }
        EvalError::UndefinedVariable { name, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Undefined variable: '{}'", name))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("variable '{}' is not defined", name))
                        .with_color(Color::Red),
                )
                .with_help(format!("Assign a value to '{}' before using it", name))
                .finish()
        }
        EvalError::UndefinedProperty { list, prop, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Undefined property: '{}.{}'", list, prop))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("property '{}' does not exist on '{}'", prop, list))
                        .with_color(Color::Red),
                )
                .with_help(format!("Check that '{}' has a '{}' property or sublist", list, prop))
                .finish()
        }
        EvalError::InvalidMethodCall { message, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Invalid method call".to_string())
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(message)
                        .with_color(Color::Red),
                )
                .finish()
        }
        EvalError::EmptyList { name, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message(format!("Cannot select from empty list: '{}'", name))
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(format!("list '{}' has no items to select from", name))
                        .with_color(Color::Red),
                )
                .with_help(format!("Add items to the '{}' list", name))
                .finish()
        }
        EvalError::TypeError { message, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Type error")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(message)
                        .with_color(Color::Red),
                )
                .finish()
        }
        EvalError::ImportError { message, span } => {
            Report::build(ReportKind::Error, source_name, span.start)
                .with_message("Import error")
                .with_label(
                    Label::new((source_name, span_to_range(*span)))
                        .with_message(message)
                        .with_color(Color::Red),
                )
                .with_help("Check that the generator name is correct and available")
                .finish()
        }
    };

    report
        .write((source_name, Source::from(source)), &mut output)
        .expect("Failed to write diagnostic");

    String::from_utf8(output).expect("Invalid UTF-8 in diagnostic output")
}

/// Combined error reporting for any interpreter error
pub fn report_interpreter_error(
    source_name: &str,
    source: &str,
    error: &crate::InterpreterError,
) -> String {
    match error {
        crate::InterpreterError::Parse(e) => report_parse_error(source_name, source, e),
        crate::InterpreterError::Compile(e) => report_compile_error(source_name, source, e),
        crate::InterpreterError::Eval(e) => report_eval_error(source_name, source, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unterminated_reference_diagnostic() {
        let source = "output\n\t[animal\n";
        let error = ParseError::UnterminatedReference {
            span: Span::new(8, 15),
        };
        let diagnostic = report_parse_error("test.perchance", source, &error);
        // Just check that the diagnostic contains the error message
        assert!(diagnostic.contains("Unterminated reference"));
        // The diagnostic may not contain the exact text due to formatting
        assert!(!diagnostic.is_empty());
    }

    #[test]
    fn test_empty_list_diagnostic() {
        let source = "mylist\n";
        let error = CompileError::EmptyList {
            name: "mylist".to_string(),
            span: Span::new(0, 6),
        };
        let diagnostic = report_compile_error("test.perchance", source, &error);
        assert!(diagnostic.contains("Empty list"));
        assert!(diagnostic.contains("mylist"));
    }
}
