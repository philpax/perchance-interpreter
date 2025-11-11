/// Comprehensive integration tests for Perchance interpreter
use perchance_interpreter::evaluate_with_seed;

#[test]
fn test_simple_list_selection() {
    let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";
    let result = evaluate_with_seed(template, 100);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "dog" || output == "cat" || output == "bird");
}

#[test]
fn test_deterministic_same_seed() {
    let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";
    let result1 = evaluate_with_seed(template, 12345).unwrap();
    let result2 = evaluate_with_seed(template, 12345).unwrap();
    assert_eq!(result1, result2);
}

#[test]
fn test_deterministic_different_seed() {
    let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";
    let result1 = evaluate_with_seed(template, 11111).unwrap();
    let result2 = evaluate_with_seed(template, 22222).unwrap();
    // With different seeds, outputs *might* differ (not guaranteed, but very likely)
    // We just verify both succeed
    assert!(result1 == "dog" || result1 == "cat" || result1 == "bird");
    assert!(result2 == "dog" || result2 == "cat" || result2 == "bird");
}

#[test]
fn test_weighted_selection() {
    let template = "item\n\tcommon^100\n\trare^1\n\noutput\n\t[item]\n";

    // Run multiple times, common should appear more often
    let mut common_count = 0;
    let mut rare_count = 0;

    for seed in 0..100 {
        let result = evaluate_with_seed(template, seed).unwrap();
        if result == "common" {
            common_count += 1;
        } else if result == "rare" {
            rare_count += 1;
        }
    }

    // Common should appear significantly more often
    assert!(common_count > rare_count * 10);
}

#[test]
fn test_inline_list() {
    let template = "output\n\t{hello|goodbye}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "hello" || output == "goodbye");
}

#[test]
fn test_inline_list_with_weights() {
    let template = "output\n\t{common^10|rare^1}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "common" || output == "rare");
}

#[test]
fn test_number_range() {
    let template = "output\n\t{1-6}\n";
    for seed in 0..20 {
        let result = evaluate_with_seed(template, seed).unwrap();
        let num: i32 = result.parse().unwrap();
        assert!(num >= 1 && num <= 6);
    }
}

#[test]
fn test_number_range_negative() {
    let template = "output\n\t{-10-10}\n";
    let result = evaluate_with_seed(template, 42).unwrap();
    let num: i32 = result.parse().unwrap();
    assert!(num >= -10 && num <= 10);
}

#[test]
fn test_letter_range() {
    let template = "output\n\t{a-z}\n";
    for seed in 0..20 {
        let result = evaluate_with_seed(template, seed).unwrap();
        assert_eq!(result.len(), 1);
        let ch = result.chars().next().unwrap();
        assert!(ch >= 'a' && ch <= 'z');
    }
}

#[test]
fn test_letter_range_uppercase() {
    let template = "output\n\t{A-Z}\n";
    let result = evaluate_with_seed(template, 42).unwrap();
    assert_eq!(result.len(), 1);
    let ch = result.chars().next().unwrap();
    assert!(ch >= 'A' && ch <= 'Z');
}

#[test]
fn test_escape_sequences() {
    // Test \s (space)
    let template = "output\n\t\\s\\shello\\s\\s\n";
    let result = evaluate_with_seed(template, 42).unwrap();
    assert_eq!(result, "  hello  ");

    // Test \t (tab)
    let template = "output\n\ta\\tb\n";
    let result = evaluate_with_seed(template, 42).unwrap();
    assert_eq!(result, "a\tb");

    // Test \[ and \]
    let template = "output\n\t\\[not a reference\\]\n";
    let result = evaluate_with_seed(template, 42).unwrap();
    assert_eq!(result, "[not a reference]");

    // Test \{ and \}
    let template = "output\n\t\\{not inline\\}\n";
    let result = evaluate_with_seed(template, 42).unwrap();
    assert_eq!(result, "{not inline}");
}

#[test]
fn test_comments() {
    let template = "// This is a comment\nanimal\n\tdog // inline comment\n\tcat\n\noutput\n\t[animal] // another comment\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should not contain "//" or "comment"
    assert!(!output.contains("//"));
    assert!(!output.contains("comment"));
}

#[test]
fn test_hierarchical_lists() {
    let template = "creature\n\tland\n\t\tdog\n\t\tcat\n\twater\n\t\tfish\n\t\twhale\n\noutput\n\t[creature]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(
        output == "dog" || output == "cat" || output == "fish" || output == "whale"
    );
}

#[test]
fn test_hierarchical_list_direct_access() {
    let template = "creature\n\tland\n\t\tdog\n\t\tcat\n\twater\n\t\tfish\n\t\twhale\n\noutput\n\t[creature.land]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "dog" || output == "cat");
}

#[test]
fn test_properties() {
    let template = "character\n\twizard\n\t\tname\n\t\t\tGandalf\n\t\t\tMerlin\n\t\tpower\n\t\t\t{80-100}\n\noutput\n\t[character.wizard.name]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "Gandalf" || output == "Merlin");
}

#[test]
fn test_variable_assignment() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[x = animal, x] and [x]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should have the same animal twice
    assert!(output == "dog and dog" || output == "cat and cat");
}

#[test]
fn test_variable_assignment_with_properties() {
    let template = "character\n\twizard\n\t\tname\n\t\t\tGandalf\n\t\ttype\n\t\t\tMagic User\n\noutput\n\t[c = character.wizard, c.name] is a [c.type]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output, "Gandalf is a Magic User");
}

#[test]
fn test_comma_sequence_with_output() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[x = animal, \"I saw a [x]\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "I saw a dog" || output == "I saw a cat");
}

#[test]
fn test_comma_sequence_no_output() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[x = animal, \"\"]Result: [x]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "Result: dog" || output == "Result: cat");
}

#[test]
fn test_method_select_one() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[animal.selectOne]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "dog" || output == "cat");
}

#[test]
fn test_method_upper_case() {
    let template = "word\n\thello\n\noutput\n\t[word.upperCase]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "HELLO");
}

#[test]
fn test_method_lower_case() {
    let template = "word\n\tHELLO\n\noutput\n\t[word.lowerCase]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn test_method_title_case() {
    let template = "phrase\n\thello world\n\noutput\n\t[phrase.titleCase]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello World");
}

#[test]
fn test_method_sentence_case() {
    let template = "phrase\n\thello world\n\noutput\n\t[phrase.sentenceCase]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello world");
}

#[test]
fn test_complex_nested_references() {
    let template = "adj\n\tbig\n\tsmall\n\nanimal\n\tdog\n\tcat\n\noutput\n\tA [adj] [animal] saw a [adj] [animal].\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Just verify it contains expected words
    assert!(output.starts_with("A "));
    assert!(output.contains(" saw a "));
}

#[test]
fn test_multiple_inline_lists() {
    let template = "output\n\t{big|small} {red|blue} {cat|dog}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    let words: Vec<&str> = output.split_whitespace().collect();
    assert_eq!(words.len(), 3);
}

#[test]
fn test_nested_inline_lists() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t{[animal]|bird}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "dog" || output == "cat" || output == "bird");
}

#[test]
fn test_mixed_content() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a {big|small} [animal] with {1-10} legs!\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.starts_with("I saw a "));
    assert!(output.contains(" legs!"));
}

#[test]
fn test_empty_output_list_error() {
    let template = "animal\n\tdog\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("output"));
}

#[test]
fn test_undefined_list_error() {
    let template = "output\n\t[nonexistent]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("nonexistent"));
}

#[test]
fn test_empty_list_error() {
    let template = "animal\n\noutput\n\t[animal]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_err());
}

#[test]
fn test_whitespace_preservation_in_text() {
    let template = "output\n\thello  world\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello  world");
}

#[test]
fn test_two_space_indentation() {
    let template = "animal\n  dog\n  cat\n\noutput\n  [animal]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output == "dog" || output == "cat");
}

#[test]
fn test_property_with_selectOne() {
    let template = "character\n\twizard\n\t\tname\n\t\t\tGandalf\n\t\tpower\n\t\t\thigh\n\noutput\n\t[c = character.selectOne, c.name]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Gandalf");
}

#[test]
fn test_number_range_in_text() {
    let template = "output\n\tRolled a {1-6} on the dice!\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.starts_with("Rolled a "));
    assert!(output.ends_with(" on the dice!"));
}

#[test]
fn test_multiple_list_references() {
    let template = "name\n\tAlice\n\tBob\n\ncity\n\tParis\n\tTokyo\n\noutput\n\t[name] lives in [city].\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains(" lives in "));
    assert!(output.ends_with("."));
}

#[test]
fn test_literal_string_in_sequence() {
    let template = "output\n\t[\"Hello World\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello World");
}

#[test]
fn test_article_consonant() {
    let template = "output\n\tI saw {a} cat.\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "I saw a cat.");
}

#[test]
fn test_article_vowel() {
    let template = "output\n\tI saw {a} elephant.\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "I saw an elephant.");
}

#[test]
fn test_article_with_reference() {
    let template = "animal\n\tapple\n\tdog\n\noutput\n\tI saw {a} [animal].\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should be either "I saw an apple." or "I saw a dog."
    assert!(output == "I saw an apple." || output == "I saw a dog.");
}

#[test]
fn test_pluralize_singular() {
    let template = "output\n\t1 apple{s}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "1 apple");
}

#[test]
fn test_pluralize_plural() {
    let template = "output\n\t3 apple{s}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "3 apples");
}

#[test]
fn test_pluralize_with_zero() {
    let template = "output\n\t0 apple{s}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "0 apples");
}

#[test]
fn test_pluralize_with_reference() {
    let template = "output\n\t{1-6} apple{s}\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should match pattern: "N apple(s)" where N is 1-6
    assert!(output.ends_with(" apple") || output.ends_with(" apples"));
}

#[test]
fn test_article_and_pluralize_combined() {
    let template = "output\n\tI want {a} {1-3} orange{s}.\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should have proper article and pluralization
    assert!(output.starts_with("I want a ") || output.starts_with("I want an "));
}

#[test]
fn test_plural_form_regular() {
    let template = "word\n\tcat\n\noutput\n\t[word.pluralForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "cats");
}

#[test]
fn test_plural_form_irregular() {
    let template = "word\n\tchild\n\noutput\n\t[word.pluralForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "children");
}

#[test]
fn test_plural_form_es() {
    let template = "word\n\tbox\n\noutput\n\t[word.pluralForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "boxes");
}

#[test]
fn test_plural_form_y_to_ies() {
    let template = "word\n\tcity\n\noutput\n\t[word.pluralForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "cities");
}

#[test]
fn test_past_tense_regular() {
    let template = "verb\n\twalk\n\noutput\n\t[verb.pastTense]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "walked");
}

#[test]
fn test_past_tense_irregular() {
    let template = "verb\n\tgo\n\noutput\n\t[verb.pastTense]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "went");
}

#[test]
fn test_past_tense_ends_with_e() {
    let template = "verb\n\tlove\n\noutput\n\t[verb.pastTense]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "loved");
}

#[test]
fn test_possessive_form() {
    let template = "name\n\tJohn\n\noutput\n\t[name.possessiveForm] book\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "John's book");
}

#[test]
fn test_possessive_form_ends_with_s() {
    let template = "name\n\tJames\n\noutput\n\t[name.possessiveForm] book\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "James' book");
}

#[test]
fn test_grammar_methods_combined() {
    let template = "noun\n\tdog\n\nverb\n\twalk\n\noutput\n\tThe [noun.pluralForm] [verb.pastTense].\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "The dogs walked.");
}

// Tests for conditional logic (ternary operator)

#[test]
fn test_ternary_operator_true() {
    let template = "output\n\t[5 > 3 ? \"yes\" : \"no\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "yes");
}

#[test]
fn test_ternary_operator_false() {
    let template = "output\n\t[2 > 5 ? \"yes\" : \"no\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "no");
}

#[test]
fn test_ternary_with_variable() {
    let template = "output\n\t[n = 3, n < 4 ? \"low\" : \"high\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "low");
}

#[test]
fn test_ternary_nested() {
    let template = "output\n\t[n = 5, n < 3 ? \"low\" : n > 7 ? \"high\" : \"mid\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "mid");
}

// Tests for binary operators

#[test]
fn test_binary_op_equal() {
    let template = "output\n\t[5 == 5 ? \"equal\" : \"not equal\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "equal");
}

#[test]
fn test_binary_op_not_equal() {
    let template = "output\n\t[5 != 3 ? \"different\" : \"same\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "different");
}

#[test]
fn test_binary_op_less_than() {
    let template = "output\n\t[3 < 5 ? \"less\" : \"not less\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "less");
}

#[test]
fn test_binary_op_greater_than() {
    let template = "output\n\t[7 > 4 ? \"greater\" : \"not greater\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "greater");
}

#[test]
fn test_binary_op_less_equal() {
    let template = "output\n\t[3 <= 3 ? \"yes\" : \"no\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "yes");
}

#[test]
fn test_binary_op_greater_equal() {
    let template = "output\n\t[5 >= 5 ? \"yes\" : \"no\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "yes");
}

#[test]
fn test_binary_op_and() {
    let template = "output\n\t[5 > 3 && 7 > 4 ? \"both\" : \"not both\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "both");
}

#[test]
fn test_binary_op_and_false() {
    let template = "output\n\t[5 > 3 && 2 > 4 ? \"both\" : \"not both\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "not both");
}

#[test]
fn test_binary_op_or() {
    let template = "output\n\t[5 > 3 || 2 > 4 ? \"at least one\" : \"neither\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "at least one");
}

#[test]
fn test_binary_op_or_false() {
    let template = "output\n\t[2 > 3 || 1 > 4 ? \"at least one\" : \"neither\"]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "neither");
}

// Tests for $output keyword

#[test]
fn test_output_keyword_simple() {
    let template = "greeting\n\thello\n\thi\n\t$output = Welcome\n\noutput\n\t[greeting]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Welcome");
}

#[test]
fn test_output_keyword_with_reference() {
    let template = "name\n\tAlice\n\tBob\n\ngreeting\n\titem\n\t$output = Hello [name]\n\noutput\n\t[greeting]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    // Should output "Hello Alice" or "Hello Bob"
    let output = result.unwrap();
    assert!(output.starts_with("Hello "));
    assert!(output == "Hello Alice" || output == "Hello Bob");
}

#[test]
fn test_output_keyword_no_items() {
    let template = "message\n\t$output = Fixed message\n\noutput\n\t[message]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Fixed message");
}

// Tests for advanced grammar methods

#[test]
fn test_future_tense() {
    let template = "verb\n\twalk\n\noutput\n\t[verb.futureTense]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "will walk");
}

#[test]
fn test_future_tense_irregular() {
    let template = "verb\n\tgo\n\noutput\n\t[verb.futureTense]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "will go");
}

#[test]
fn test_present_tense_from_past() {
    let template = "verb\n\twent\n\noutput\n\t[verb.presentTense]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "goes");
}

#[test]
fn test_present_tense_regular() {
    let template = "verb\n\twalk\n\noutput\n\t[verb.presentTense]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "walks");
}

#[test]
fn test_negative_form() {
    let template = "verb\n\texamine\n\noutput\n\t[verb.negativeForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "does not examine");
}

#[test]
fn test_negative_form_be() {
    let template = "verb\n\tis\n\noutput\n\t[verb.negativeForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "is not");
}

#[test]
fn test_singular_form() {
    let template = "word\n\tcities\n\noutput\n\t[word.singularForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "city");
}

#[test]
fn test_singular_form_irregular() {
    let template = "word\n\tchildren\n\noutput\n\t[word.singularForm]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "child");
}

// Tests for joinItems method

#[test]
fn test_join_items_with_comma() {
    let template = "fruit\n\tapple\n\tbanana\n\torange\n\noutput\n\t[fruit.selectMany(3).joinItems(\", \")]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should have comma separators
    assert!(output.contains(", "));
    // Should have 3 items (2 commas)
    assert_eq!(output.matches(", ").count(), 2);
}

#[test]
fn test_join_items_with_custom_separator() {
    let template = "word\n\tfoo\n\tbar\n\tbaz\n\noutput\n\t[word.selectMany(2).joinItems(\" | \")]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should have custom separator
    assert!(output.contains(" | "));
}

#[test]
fn test_join_items_select_unique() {
    let template = "color\n\tred\n\tblue\n\tgreen\n\noutput\n\t[color.selectUnique(2).joinItems(\" and \")]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should have " and " separator
    assert!(output.contains(" and "));
    // Should have exactly one " and " (2 items)
    assert_eq!(output.matches(" and ").count(), 1);
}

#[test]
fn test_join_items_default_separator() {
    let template = "num\n\t1\n\t2\n\t3\n\noutput\n\t[num.selectMany(3)]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Default separator is space
    assert!(output.contains(" "));
}

// Tests for consumableList
#[test]
fn test_consumable_list_basic() {
    let template = "item\n\ta\n\tb\n\tc\n\noutput\n\t[c = item.consumableList][c] [c] [c]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should have 3 items (space-separated)
    assert_eq!(output.split_whitespace().count(), 3);
    // Each item should be one of a, b, c
    let parts: Vec<&str> = output.split_whitespace().collect();
    for part in &parts {
        assert!(part == &"a" || part == &"b" || part == &"c");
    }
}

#[test]
fn test_consumable_list_exhaustion() {
    let template = "item\n\ta\n\tb\n\noutput\n\t[c = item.consumableList][c] [c] [c]\n";
    let result = evaluate_with_seed(template, 42);
    // Should fail because we try to consume 3 items from a 2-item list
    assert!(result.is_err());
}

#[test]
fn test_consumable_list_select_unique() {
    let template = "item\n\ta\n\tb\n\tc\n\td\n\noutput\n\t[item.consumableList.selectUnique(3).joinItems(\", \")]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    // Should have 3 unique items
    let parts: Vec<&str> = output.split(", ").collect();
    assert_eq!(parts.len(), 3);
    // All items should be unique
    let mut sorted = parts.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 3);
}

#[test]
fn test_consumable_list_no_duplicates() {
    // Test that consumableList doesn't repeat items until exhausted
    let template = "item\n\ta\n\tb\n\tc\n\noutput\n\t[c = item.consumableList][c], [c], [c]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    let output = result.unwrap();
    let parts: Vec<&str> = output.split(", ").collect();
    assert_eq!(parts.len(), 3);
    // All three should be different
    let mut sorted = parts.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 3, "Expected all items to be unique, got: {}", output);
}

#[test]
fn test_consumable_list_independent_instances() {
    // Test that different consumableList instances are independent
    let template = "item\n\ta\n\tb\n\tc\n\noutput\n\t[c1 = item.consumableList][c2 = item.consumableList][c1] [c2]\n";
    let result = evaluate_with_seed(template, 42);
    assert!(result.is_ok());
    // Both c1 and c2 should work independently
}
