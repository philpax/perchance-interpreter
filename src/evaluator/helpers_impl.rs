//! Helper functions and methods for the Evaluator
//!
//! This module contains utility methods used throughout the evaluator:
//! - `get_item_preview` - Create a preview string from item content
//! - `get_expr_preview` - Create a preview string from an expression
//! - `extract_number` - Extract numeric values from text
//! - `peek_next_word` - Look ahead to get the next word in content
//! - `starts_with_vowel_sound` - Check if a word starts with a vowel
//! - `is_truthy` - Determine truthiness of a string value
//! - `compare_values` - Compare two values for ordering
//! - `format_number` - Format numeric values appropriately

use crate::ast::{ContentPart, Expression};
use crate::span::Spanned;
use rand::Rng;

use super::{EvalError, Evaluator};

/// Get a simple preview of an expression
/// This is a standalone function, not a method, as indicated by the original code
fn get_expr_preview(expr: &Expression) -> String {
    match expr {
        Expression::Simple(ident) => ident.value.name.clone(),
        Expression::Property(base, prop) => {
            format!("{}.{}", get_expr_preview(&base.value), prop.value.name)
        }
        Expression::Import(name) => format!("import:{}", name),
        Expression::Literal(s) => format!("\"{}\"", s),
        Expression::Number(n) => n.to_string(),
        Expression::NumberRange(a, b) => format!("{{{}-{}}}", a, b),
        _ => "...".to_string(),
    }
}

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    /// Get a preview of item content for display/debugging purposes
    ///
    /// Creates a truncated string representation of the first few content parts,
    /// useful for showing what an item contains without full evaluation.
    pub(super) fn get_item_preview(&self, content: &[Spanned<ContentPart>]) -> String {
        let mut preview = String::new();
        for part in content.iter().take(3) {
            // Limit to first 3 parts
            match &part.value {
                ContentPart::Text(text) => preview.push_str(text),
                ContentPart::Reference(expr) => {
                    preview.push_str(&format!("[{}]", get_expr_preview(&expr.value)))
                }
                ContentPart::Inline(_) => preview.push_str("{...}"),
                ContentPart::Article => preview.push_str("{a}"),
                ContentPart::Pluralize => preview.push_str("{s}"),
                ContentPart::Escape(ch) => preview.push(*ch),
            }
            if preview.len() > 50 {
                preview.truncate(50);
                preview.push_str("...");
                break;
            }
        }
        if preview.is_empty() {
            preview = "(empty)".to_string();
        }
        preview
    }

    /// Extract a numeric value from text, returning the last number found
    ///
    /// Scans through space-separated words and extracts any numeric values.
    /// Returns the last number found, or None if no valid numbers exist.
    pub(super) fn extract_number(&self, text: &str) -> Option<i64> {
        // Look for any number in the text (last one wins)
        let mut last_num = None;
        for word in text.split_whitespace() {
            if let Ok(num) = word
                .trim_matches(|c: char| !c.is_ascii_digit() && c != '-')
                .parse::<i64>()
            {
                last_num = Some(num);
            }
        }
        last_num
    }

    /// Peek at the next word in content without consuming it
    ///
    /// Looks ahead from `start_idx` to find the first non-empty word from:
    /// - Text parts (returns first word)
    /// - References (evaluates and returns first word)
    /// - Inline expressions (evaluates and returns first word)
    ///
    /// Returns "word" as default if no words are found.
    pub(super) async fn peek_next_word(
        &mut self,
        content: &[Spanned<ContentPart>],
        start_idx: usize,
    ) -> Result<String, EvalError> {
        for part_spanned in &content[start_idx..] {
            match &part_spanned.value {
                ContentPart::Text(text) => {
                    // Get the first word from the text
                    if let Some(word) = text.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                ContentPart::Reference(expr_spanned) => {
                    // Evaluate the reference to get the word
                    let value = self.evaluate_expression(expr_spanned).await?;
                    if let Some(word) = value.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                ContentPart::Inline(inline_spanned) => {
                    // Evaluate the inline to get the word
                    let value = self.evaluate_inline(inline_spanned).await?;
                    if let Some(word) = value.split_whitespace().next() {
                        if !word.is_empty() {
                            return Ok(word.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        // If no word found, default to consonant article "a"
        Ok(String::from("word"))
    }

    /// Check if a word starts with a vowel sound
    ///
    /// Performs a simple vowel check based on the first character.
    /// Could be enhanced with special cases like "hour" (vowel sound)
    /// or "university" (consonant sound).
    pub(super) fn starts_with_vowel_sound(&self, word: &str) -> bool {
        if word.is_empty() {
            return false;
        }

        let first_char = word.chars().next().unwrap().to_ascii_lowercase();

        // Simple vowel check - could be enhanced with special cases
        // (e.g., "hour" starts with vowel sound, "university" doesn't)
        matches!(first_char, 'a' | 'e' | 'i' | 'o' | 'u')
    }

    /// Determine if a string value is truthy
    ///
    /// Empty string, "false", and "0" are considered falsy.
    /// All other values are truthy.
    pub(super) fn is_truthy(&self, s: &str) -> bool {
        // Empty string, "false", "0" are falsy
        !s.is_empty() && s != "false" && s != "0"
    }

    /// Compare two string values, treating them as numbers if possible
    ///
    /// First attempts to parse both values as floats. If successful,
    /// performs numeric comparison. Otherwise, performs string comparison.
    ///
    /// Returns:
    /// - `-1` if left < right
    /// - `0` if left == right
    /// - `1` if left > right
    pub(super) fn compare_values(&self, left: &str, right: &str) -> Result<i32, EvalError> {
        // Try to parse as numbers first
        if let (Ok(l), Ok(r)) = (left.parse::<f64>(), right.parse::<f64>()) {
            if l < r {
                Ok(-1)
            } else if l > r {
                Ok(1)
            } else {
                Ok(0)
            }
        } else {
            // String comparison
            Ok(left.cmp(right) as i32)
        }
    }

    /// Format a numeric value appropriately for output
    ///
    /// Formats integers without decimal points, and floats with standard formatting.
    /// Integers are detected when the fractional part is zero and the value
    /// is not too large.
    pub(super) fn format_number(&self, num: f64) -> String {
        // Format number without unnecessary decimal points
        if num.fract() == 0.0 && num.abs() < 1e15 {
            // It's an integer (or very close to one)
            format!("{}", num as i64)
        } else {
            // It's a float, format with precision
            format!("{}", num)
        }
    }
}
