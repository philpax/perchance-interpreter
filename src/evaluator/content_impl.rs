/// Content evaluation implementation for the Evaluator
use crate::ast::{ContentPart, Expression, InlineList, ItemWeight};
use crate::span::Spanned;
use crate::trace::OperationType;
use async_recursion::async_recursion;
use rand::Rng;

use super::{EvalError, Evaluator};

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    /// Evaluate content parts (text, references, inline lists) into a string
    pub(super) async fn evaluate_content(
        &mut self,
        content: &[Spanned<ContentPart>],
    ) -> Result<String, EvalError> {
        let mut result = String::new();

        for (i, part_spanned) in content.iter().enumerate() {
            let part = &part_spanned.value;
            match part {
                ContentPart::Text(text) => {
                    result.push_str(text);
                    // Track numbers for {s} pluralization
                    if let Some(num) = self.extract_number(text) {
                        self.last_number = Some(num);
                    }
                }
                ContentPart::Escape(ch) => result.push(*ch),
                ContentPart::Reference(expr_spanned) => {
                    let value = self.evaluate_expression(expr_spanned).await?;
                    // Track numbers for {s} pluralization
                    if let Ok(num) = value.parse::<i64>() {
                        self.last_number = Some(num);
                    }
                    result.push_str(&value);
                }
                ContentPart::Inline(inline_spanned) => {
                    let inline = &inline_spanned.value;
                    // Check if this is a special inline: {a} or {s}
                    if inline.choices.len() == 1 && inline.choices[0].value.content.len() == 1 {
                        match &inline.choices[0].value.content[0].value {
                            ContentPart::Article => {
                                // {a} - choose "a" or "an" based on next word
                                let next_word = self.peek_next_word(content, i + 1).await?;
                                if self.starts_with_vowel_sound(&next_word) {
                                    result.push_str("an");
                                } else {
                                    result.push('a');
                                }
                                continue;
                            }
                            ContentPart::Pluralize => {
                                // {s} - add "s" if last number != 1
                                if let Some(num) = self.last_number {
                                    if num != 1 && num != -1 {
                                        result.push('s');
                                    }
                                } else {
                                    // Default to plural if no number context
                                    result.push('s');
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Regular inline evaluation
                    let value = self.evaluate_inline(inline_spanned).await?;
                    // Track numbers for {s} pluralization
                    if let Ok(num) = value.parse::<i64>() {
                        self.last_number = Some(num);
                    }
                    result.push_str(&value);
                }
                ContentPart::Article => {
                    // {a} - choose "a" or "an" based on next word
                    let next_word = self.peek_next_word(content, i + 1).await?;
                    if self.starts_with_vowel_sound(&next_word) {
                        result.push_str("an");
                    } else {
                        result.push('a');
                    }
                }
                ContentPart::Pluralize => {
                    // {s} - add "s" if last number != 1
                    if let Some(num) = self.last_number {
                        if num != 1 && num != -1 {
                            result.push('s');
                        }
                    } else {
                        // Default to plural if no number context
                        result.push('s');
                    }
                }
            }
        }

        Ok(result)
    }

    /// Evaluate an inline list (weighted random selection)
    #[async_recursion]
    pub(super) async fn evaluate_inline(
        &mut self,
        inline_spanned: &Spanned<InlineList>,
    ) -> Result<String, EvalError> {
        let inline = &inline_spanned.value;
        if inline.choices.is_empty() {
            return Ok(String::new());
        }

        // Start tracing
        self.trace_start(
            "{...}".to_string(),
            OperationType::Choice,
            Some(inline_spanned.span),
        );

        // Check if this is a special case (number range, letter range)
        if inline.choices.len() == 1 {
            if let Some(content_part_spanned) = inline.choices[0].value.content.first() {
                if let ContentPart::Reference(expr_spanned) = &content_part_spanned.value {
                    match &expr_spanned.value {
                        Expression::NumberRange(start, end) => {
                            let num = self.rng.gen_range(*start..=*end);
                            let result = num.to_string();

                            // Store inline list content for tracing
                            if self.trace_enabled {
                                if let Some(node) = self.trace_stack.last_mut() {
                                    node.inline_list_content =
                                        Some(format!("{{{}-{}}}", start, end));
                                }
                            }

                            self.trace_end(result.clone());
                            return Ok(result);
                        }
                        Expression::LetterRange(start, end) => {
                            let start_byte = *start as u8;
                            let end_byte = *end as u8;
                            let random_byte = self.rng.gen_range(start_byte..=end_byte);
                            let result = (random_byte as char).to_string();

                            // Store inline list content for tracing
                            if self.trace_enabled {
                                if let Some(node) = self.trace_stack.last_mut() {
                                    node.inline_list_content =
                                        Some(format!("{{{}-{}}}", start, end));
                                }
                            }

                            self.trace_end(result.clone());
                            return Ok(result);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Build inline list content string for tracing
        let inline_content = if self.trace_enabled {
            let mut parts = Vec::new();
            for (i, choice_spanned) in inline.choices.iter().enumerate() {
                let choice = &choice_spanned.value;
                let preview = self.get_item_preview(&choice.content);
                let weight_str = match &choice.weight {
                    Some(ItemWeight::Static(w)) if (*w - 1.0).abs() > 0.001 => format!("^{}", w),
                    _ => String::new(),
                };
                parts.push(format!("{}{}", preview, weight_str));
                if i < inline.choices.len() - 1 {
                    parts.push("|".to_string());
                }
            }
            Some(parts.join(" "))
        } else {
            None
        };

        // Calculate actual weights for choices with dynamic weights
        let mut actual_weights: Vec<f64> = Vec::new();
        let mut actual_total = 0.0;

        for choice_spanned in &inline.choices {
            let choice = &choice_spanned.value;
            let weight = match &choice.weight {
                Some(ItemWeight::Static(w)) => *w,
                Some(ItemWeight::Dynamic(expr_spanned)) => {
                    // Evaluate the dynamic weight expression
                    let result = self.evaluate_expression(expr_spanned).await?;
                    // Convert to number: "true" -> 1.0, "false" -> 0.0, or parse as number
                    let weight = if result == "true" {
                        1.0
                    } else if result == "false" || result.is_empty() {
                        0.0
                    } else {
                        result.parse::<f64>().unwrap_or(0.0)
                    };
                    weight.max(0.0)
                }
                None => 1.0,
            };
            actual_weights.push(weight);
            actual_total += weight;
        }

        if actual_total <= 0.0 {
            // If all weights are 0, treat all choices as having equal weight (1.0)
            actual_weights = vec![1.0; inline.choices.len()];
            actual_total = inline.choices.len() as f64;
        }

        // Generate choice previews for tracing
        let choice_previews: Vec<String> = if self.trace_enabled {
            inline
                .choices
                .iter()
                .map(|choice| self.get_item_preview(&choice.value.content))
                .collect()
        } else {
            Vec::new()
        };

        // Select a choice
        let random_value = self.rng.gen::<f64>() * actual_total;
        let mut cumulative = 0.0;
        let mut selected_idx = inline.choices.len() - 1;

        for (i, weight) in actual_weights.iter().enumerate() {
            cumulative += weight;
            if random_value < cumulative {
                selected_idx = i;
                break;
            }
        }

        // Store trace information
        if self.trace_enabled {
            if let Some(node) = self.trace_stack.last_mut() {
                node.available_items = Some(choice_previews);
                node.selected_index = Some(selected_idx);
                node.inline_list_content = inline_content;
            }
        }

        // Evaluate the selected choice
        let result = self
            .evaluate_content(&inline.choices[selected_idx].value.content)
            .await?;

        self.trace_end(result.clone());
        Ok(result)
    }
}
