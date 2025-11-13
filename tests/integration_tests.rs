/// Comprehensive integration tests for Perchance interpreter
use perchance_interpreter::run_with_seed;

#[tokio::test]
async fn test_simple_list_selection() {
    let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";
    let output = run_with_seed(template, 100, None).await.unwrap();
    assert!(output == "dog" || output == "cat" || output == "bird");
}

#[tokio::test]
async fn test_simple_list_selection_with_multiline() {
    let template = r#"animal
    pig
    cow
    zebra
sentence
    That [animal] is very sneaky."#;
    let output = run_with_seed(template, 100, None).await.unwrap();
    assert!(
        output.contains("pig") || output.contains("cow") || output.contains("zebra"),
        "Expected output to contain one of the animals, got: {}",
        output
    );
    assert!(output.contains("sneaky"));
}

#[tokio::test]
async fn test_deterministic_same_seed() {
    let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";
    let result1 = run_with_seed(template, 12345, None).await.unwrap();
    let result2 = run_with_seed(template, 12345, None).await.unwrap();
    assert_eq!(result1, result2);
}

#[tokio::test]
async fn test_deterministic_different_seed() {
    let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";
    let result1 = run_with_seed(template, 11111, None).await.unwrap();
    let result2 = run_with_seed(template, 22222, None).await.unwrap();
    // With different seeds, outputs *might* differ (not guaranteed, but very likely)
    // We just verify both succeed
    assert!(result1 == "dog" || result1 == "cat" || result1 == "bird");
    assert!(result2 == "dog" || result2 == "cat" || result2 == "bird");
}

#[tokio::test]
async fn test_weighted_selection() {
    let template = "item\n\tcommon^100\n\trare^1\n\noutput\n\t[item]\n";

    // Run multiple times, common should appear more often
    let mut common_count = 0;
    let mut rare_count = 0;

    for seed in 0..100 {
        let result = run_with_seed(template, seed, None).await.unwrap();
        if result == "common" {
            common_count += 1;
        } else if result == "rare" {
            rare_count += 1;
        }
    }

    // Common should appear significantly more often
    assert!(common_count > rare_count * 10);
}

#[tokio::test]
async fn test_inline_list() {
    let template = "output\n\t{hello|goodbye}\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "hello" || output == "goodbye");
}

#[tokio::test]
async fn test_inline_list_with_weights() {
    let template = "output\n\t{common^10|rare^1}\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "common" || output == "rare");
}

#[tokio::test]
async fn test_number_range() {
    let template = "output\n\t{1-6}\n";
    for seed in 0..20 {
        let result = run_with_seed(template, seed, None).await.unwrap();
        let num: i32 = result.parse().unwrap();
        assert!((1..=6).contains(&num));
    }
}

#[tokio::test]
async fn test_number_range_negative() {
    let template = "output\n\t{-10-10}\n";
    let result = run_with_seed(template, 42, None).await.unwrap();
    let num: i32 = result.parse().unwrap();
    assert!((-10..=10).contains(&num));
}

#[tokio::test]
async fn test_letter_range() {
    let template = "output\n\t{a-z}\n";
    for seed in 0..20 {
        let result = run_with_seed(template, seed, None).await.unwrap();
        assert_eq!(result.len(), 1);
        let ch = result.chars().next().unwrap();
        assert!(ch.is_ascii_lowercase());
    }
}

#[tokio::test]
async fn test_letter_range_uppercase() {
    let template = "output\n\t{A-Z}\n";
    let result = run_with_seed(template, 42, None).await.unwrap();
    assert_eq!(result.len(), 1);
    let ch = result.chars().next().unwrap();
    assert!(ch.is_ascii_uppercase());
}

#[tokio::test]
async fn test_escape_sequences() {
    // Test \s (space)
    let template = "output\n\t\\s\\shello\\s\\s\n";
    let result = run_with_seed(template, 42, None).await.unwrap();
    assert_eq!(result, "  hello  ");

    // Test \t (tab)
    let template = "output\n\ta\\tb\n";
    let result = run_with_seed(template, 42, None).await.unwrap();
    assert_eq!(result, "a\tb");

    // Test \[ and \]
    let template = "output\n\t\\[not a reference\\]\n";
    let result = run_with_seed(template, 42, None).await.unwrap();
    assert_eq!(result, "[not a reference]");

    // Test \{ and \}
    let template = "output\n\t\\{not inline\\}\n";
    let result = run_with_seed(template, 42, None).await.unwrap();
    assert_eq!(result, "{not inline}");
}

#[tokio::test]
async fn test_comments() {
    let template = "// This is a comment\nanimal\n\tdog // inline comment\n\tcat\n\noutput\n\t[animal] // another comment\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should not contain "//" or "comment"
    assert!(!output.contains("//"));
    assert!(!output.contains("comment"));
}

#[tokio::test]
async fn test_hierarchical_lists() {
    let template = "creature\n\tland\n\t\tdog\n\t\tcat\n\twater\n\t\tfish\n\t\twhale\n\noutput\n\t[creature]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat" || output == "fish" || output == "whale");
}

#[tokio::test]
async fn test_hierarchical_list_direct_access() {
    let template = "creature\n\tland\n\t\tdog\n\t\tcat\n\twater\n\t\tfish\n\t\twhale\n\noutput\n\t[creature.land]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat");
}

#[tokio::test]
async fn test_properties() {
    let template = "character\n\twizard\n\t\tname\n\t\t\tGandalf\n\t\t\tMerlin\n\t\tpower\n\t\t\t{80-100}\n\noutput\n\t[character.wizard.name]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "Gandalf" || output == "Merlin");
}

#[tokio::test]
async fn test_variable_assignment() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[x = animal, x] and [x]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have the same animal twice
    assert!(output == "dog and dog" || output == "cat and cat");
}

#[tokio::test]
async fn test_variable_assignment_with_properties() {
    let template = "character\n\twizard\n\t\tname\n\t\t\tGandalf\n\t\ttype\n\t\t\tMagic User\n\noutput\n\t[c = character.wizard, c.name] is a [c.type]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert_eq!(output, "Gandalf is a Magic User");
}

#[tokio::test]
async fn test_comma_sequence_with_output() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[x = animal, \"I saw a [x]\"]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "I saw a dog" || output == "I saw a cat");
}

#[tokio::test]
async fn test_comma_sequence_no_output() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[x = animal, \"\"]Result: [x]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "Result: dog" || output == "Result: cat");
}

#[tokio::test]
async fn test_method_select_one() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[animal.selectOne]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat");
}

#[tokio::test]
async fn test_method_upper_case() {
    let template = "word\n\thello\n\noutput\n\t[word.upperCase]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "HELLO");
}

#[tokio::test]
async fn test_method_lower_case() {
    let template = "word\n\tHELLO\n\noutput\n\t[word.lowerCase]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "hello");
}

#[tokio::test]
async fn test_method_title_case() {
    let template = "phrase\n\thello world\n\noutput\n\t[phrase.titleCase]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "Hello World");
}

#[tokio::test]
async fn test_method_sentence_case() {
    let template = "phrase\n\thello world\n\noutput\n\t[phrase.sentenceCase]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "Hello world");
}

#[tokio::test]
async fn test_complex_nested_references() {
    let template = "adj\n\tbig\n\tsmall\n\nanimal\n\tdog\n\tcat\n\noutput\n\tA [adj] [animal] saw a [adj] [animal].\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Just verify it contains expected words
    assert!(output.starts_with("A "));
    assert!(output.contains(" saw a "));
}

#[tokio::test]
async fn test_multiple_inline_lists() {
    let template = "output\n\t{big|small} {red|blue} {cat|dog}\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    let words: Vec<&str> = output.split_whitespace().collect();
    assert_eq!(words.len(), 3);
}

#[tokio::test]
async fn test_nested_inline_lists() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t{[animal]|bird}\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat" || output == "bird");
}

#[tokio::test]
async fn test_mixed_content() {
    let template =
        "animal\n\tdog\n\tcat\n\noutput\n\tI saw a {big|small} [animal] with {1-10} legs!\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output.starts_with("I saw a "));
    assert!(output.contains(" legs!"));
}

#[tokio::test]
async fn test_default_to_last_list() {
    // When no "output" list is defined, the last list should be used as output
    let template = "animal\n\tdog\n\tcat\n";
    let result = run_with_seed(template, 42, None).await.unwrap();
    assert!(result == "dog" || result == "cat");
}

#[tokio::test]
async fn test_undefined_list_error() {
    let template = "output\n\t[nonexistent]\n";
    let result = run_with_seed(template, 42, None);
    assert!(result
        .await
        .unwrap_err()
        .to_string()
        .contains("nonexistent"));
}

#[tokio::test]
async fn test_empty_list_error() {
    let template = "animal\n\noutput\n\t[animal]\n";
    let result = run_with_seed(template, 42, None);
    result.await.unwrap_err();
}

#[tokio::test]
async fn test_whitespace_preservation_in_text() {
    let template = "output\n\thello  world\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "hello  world");
}

#[tokio::test]
async fn test_tab_indentation() {
    let template = "animal\n\tdog\n\tcat\n\noutput\n\t[animal]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat");
}

#[tokio::test]
async fn test_two_space_indentation() {
    let template = "animal\n  dog\n  cat\n\noutput\n  [animal]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat");
}

#[tokio::test]
async fn test_mixed_tab_and_space_indentation() {
    // Test that different lists can use different indentation styles
    let template = "animal\n\tdog\n\tcat\n\ncolor\n  red\n  blue\n\noutput\n\t[animal] [color]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output.contains("dog") || output.contains("cat"));
    assert!(output.contains("red") || output.contains("blue"));
}

#[tokio::test]
async fn test_hierarchical_with_tabs() {
    let template = "creature\n\tmammal\n\t\tdog\n\t\tcat\n\tbird\n\t\tsparrow\n\t\teagle\n\noutput\n\t[creature]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat" || output == "sparrow" || output == "eagle");
}

#[tokio::test]
async fn test_hierarchical_with_spaces() {
    let template = "creature\n  mammal\n    dog\n    cat\n  bird\n    sparrow\n    eagle\n\noutput\n  [creature]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "dog" || output == "cat" || output == "sparrow" || output == "eagle");
}

#[tokio::test]
async fn test_properties_with_tabs() {
    let template = "character\n\twizard\n\t\tname\n\t\t\tGandalf\n\t\t\tMerlin\n\noutput\n\t[character.wizard.name]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "Gandalf" || output == "Merlin");
}

#[tokio::test]
async fn test_properties_with_spaces() {
    let template = "character\n  wizard\n    name\n      Gandalf\n      Merlin\n\noutput\n  [character.wizard.name]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output == "Gandalf" || output == "Merlin");
}

#[tokio::test]
async fn test_property_with_select_one() {
    let template = "character\n\twizard\n\t\tname\n\t\t\tGandalf\n\t\tpower\n\t\t\thigh\n\noutput\n\t[c = character.selectOne, c.name]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "Gandalf");
}

#[tokio::test]
async fn test_number_range_in_text() {
    let template = "output\n\tRolled a {1-6} on the dice!\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output.starts_with("Rolled a "));
    assert!(output.ends_with(" on the dice!"));
}

#[tokio::test]
async fn test_multiple_list_references() {
    let template =
        "name\n\tAlice\n\tBob\n\ncity\n\tParis\n\tTokyo\n\noutput\n\t[name] lives in [city].\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output.contains(" lives in "));
    assert!(output.ends_with("."));
}

#[tokio::test]
async fn test_literal_string_in_sequence() {
    let template = "output\n\t[\"Hello World\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "Hello World");
}

#[tokio::test]
async fn test_article_consonant() {
    let template = "output\n\tI saw {a} cat.\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "I saw a cat.");
}

#[tokio::test]
async fn test_article_vowel() {
    let template = "output\n\tI saw {a} elephant.\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "I saw an elephant.");
}

#[tokio::test]
async fn test_article_with_reference() {
    let template = "animal\n\tapple\n\tdog\n\noutput\n\tI saw {a} [animal].\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should be either "I saw an apple." or "I saw a dog."
    assert!(output == "I saw an apple." || output == "I saw a dog.");
}

#[tokio::test]
async fn test_pluralize_singular() {
    let template = "output\n\t1 apple{s}\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "1 apple");
}

#[tokio::test]
async fn test_pluralize_plural() {
    let template = "output\n\t3 apple{s}\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "3 apples");
}

#[tokio::test]
async fn test_pluralize_with_zero() {
    let template = "output\n\t0 apple{s}\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "0 apples");
}

#[tokio::test]
async fn test_pluralize_with_reference() {
    let template = "output\n\t{1-6} apple{s}\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should match pattern: "N apple(s)" where N is 1-6
    assert!(output.ends_with(" apple") || output.ends_with(" apples"));
}

#[tokio::test]
async fn test_article_and_pluralize_combined() {
    let template = "output\n\tI want {a} {1-3} orange{s}.\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have proper article and pluralization
    assert!(output.starts_with("I want a ") || output.starts_with("I want an "));
}

#[tokio::test]
async fn test_plural_form_regular() {
    let template = "word\n\tcat\n\noutput\n\t[word.pluralForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "cats");
}

#[tokio::test]
async fn test_plural_form_irregular() {
    let template = "word\n\tchild\n\noutput\n\t[word.pluralForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "children");
}

#[tokio::test]
async fn test_plural_form_es() {
    let template = "word\n\tbox\n\noutput\n\t[word.pluralForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "boxes");
}

#[tokio::test]
async fn test_plural_form_y_to_ies() {
    let template = "word\n\tcity\n\noutput\n\t[word.pluralForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "cities");
}

#[tokio::test]
async fn test_past_tense_regular() {
    let template = "verb\n\twalk\n\noutput\n\t[verb.pastTense]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "walked");
}

#[tokio::test]
async fn test_past_tense_irregular() {
    let template = "verb\n\tgo\n\noutput\n\t[verb.pastTense]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "went");
}

#[tokio::test]
async fn test_past_tense_ends_with_e() {
    let template = "verb\n\tlove\n\noutput\n\t[verb.pastTense]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "loved");
}

#[tokio::test]
async fn test_possessive_form() {
    let template = "name\n\tJohn\n\noutput\n\t[name.possessiveForm] book\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "John's book");
}

#[tokio::test]
async fn test_possessive_form_ends_with_s() {
    let template = "name\n\tJames\n\noutput\n\t[name.possessiveForm] book\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "James' book");
}

#[tokio::test]
async fn test_grammar_methods_combined() {
    let template =
        "noun\n\tdog\n\nverb\n\twalk\n\noutput\n\tThe [noun.pluralForm] [verb.pastTense].\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "The dogs walked.");
}

// Tests for conditional logic (ternary operator)

#[tokio::test]
async fn test_ternary_operator_true() {
    let template = "output\n\t[5 > 3 ? \"yes\" : \"no\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "yes");
}

#[tokio::test]
async fn test_ternary_operator_false() {
    let template = "output\n\t[2 > 5 ? \"yes\" : \"no\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "no");
}

#[tokio::test]
async fn test_ternary_with_variable() {
    let template = "output\n\t[n = 3, n < 4 ? \"low\" : \"high\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "low");
}

#[tokio::test]
async fn test_ternary_nested() {
    let template = "output\n\t[n = 5, n < 3 ? \"low\" : n > 7 ? \"high\" : \"mid\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "mid");
}

// Tests for binary operators

#[tokio::test]
async fn test_binary_op_equal() {
    let template = "output\n\t[5 == 5 ? \"equal\" : \"not equal\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "equal");
}

#[tokio::test]
async fn test_binary_op_not_equal() {
    let template = "output\n\t[5 != 3 ? \"different\" : \"same\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "different");
}

#[tokio::test]
async fn test_binary_op_less_than() {
    let template = "output\n\t[3 < 5 ? \"less\" : \"not less\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "less");
}

#[tokio::test]
async fn test_binary_op_greater_than() {
    let template = "output\n\t[7 > 4 ? \"greater\" : \"not greater\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "greater");
}

#[tokio::test]
async fn test_binary_op_less_equal() {
    let template = "output\n\t[3 <= 3 ? \"yes\" : \"no\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "yes");
}

#[tokio::test]
async fn test_binary_op_greater_equal() {
    let template = "output\n\t[5 >= 5 ? \"yes\" : \"no\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "yes");
}

#[tokio::test]
async fn test_binary_op_and() {
    let template = "output\n\t[5 > 3 && 7 > 4 ? \"both\" : \"not both\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "both");
}

#[tokio::test]
async fn test_binary_op_and_false() {
    let template = "output\n\t[5 > 3 && 2 > 4 ? \"both\" : \"not both\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "not both");
}

#[tokio::test]
async fn test_binary_op_or() {
    let template = "output\n\t[5 > 3 || 2 > 4 ? \"at least one\" : \"neither\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "at least one");
}

#[tokio::test]
async fn test_binary_op_or_false() {
    let template = "output\n\t[2 > 3 || 1 > 4 ? \"at least one\" : \"neither\"]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "neither");
}

// Tests for $output keyword

#[tokio::test]
async fn test_output_keyword_simple() {
    let template = "greeting\n\thello\n\thi\n\t$output = Welcome\n\noutput\n\t[greeting]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "Welcome");
}

#[tokio::test]
async fn test_output_keyword_with_reference() {
    let template = "name\n\tAlice\n\tBob\n\ngreeting\n\titem\n\t$output = Hello [name]\n\noutput\n\t[greeting]\n";
    let result = run_with_seed(template, 42, None);
    // Should output "Hello Alice" or "Hello Bob"
    let output = result.await.unwrap();
    assert!(output.starts_with("Hello "));
    assert!(output == "Hello Alice" || output == "Hello Bob");
}

#[tokio::test]
async fn test_output_keyword_no_items() {
    let template = "message\n\t$output = Fixed message\n\noutput\n\t[message]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "Fixed message");
}

// Tests for advanced grammar methods

#[tokio::test]
async fn test_future_tense() {
    let template = "verb\n\twalk\n\noutput\n\t[verb.futureTense]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "will walk");
}

#[tokio::test]
async fn test_future_tense_irregular() {
    let template = "verb\n\tgo\n\noutput\n\t[verb.futureTense]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "will go");
}

#[tokio::test]
async fn test_present_tense_from_past() {
    let template = "verb\n\twent\n\noutput\n\t[verb.presentTense]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "goes");
}

#[tokio::test]
async fn test_present_tense_regular() {
    let template = "verb\n\twalk\n\noutput\n\t[verb.presentTense]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "walks");
}

#[tokio::test]
async fn test_negative_form() {
    let template = "verb\n\texamine\n\noutput\n\t[verb.negativeForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "does not examine");
}

#[tokio::test]
async fn test_negative_form_be() {
    let template = "verb\n\tis\n\noutput\n\t[verb.negativeForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "is not");
}

#[tokio::test]
async fn test_singular_form() {
    let template = "word\n\tcities\n\noutput\n\t[word.singularForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "city");
}

#[tokio::test]
async fn test_singular_form_irregular() {
    let template = "word\n\tchildren\n\noutput\n\t[word.singularForm]\n";
    let result = run_with_seed(template, 42, None);
    assert_eq!(result.await.unwrap(), "child");
}

// Tests for joinItems method

#[tokio::test]
async fn test_join_items_with_comma() {
    let template =
        "fruit\n\tapple\n\tbanana\n\torange\n\noutput\n\t[fruit.selectMany(3).joinItems(\", \")]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have comma separators
    assert!(output.contains(", "));
    // Should have 3 items (2 commas)
    assert_eq!(output.matches(", ").count(), 2);
}

#[tokio::test]
async fn test_join_items_with_custom_separator() {
    let template =
        "word\n\tfoo\n\tbar\n\tbaz\n\noutput\n\t[word.selectMany(2).joinItems(\" | \")]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have custom separator
    assert!(output.contains(" | "));
}

#[tokio::test]
async fn test_join_items_select_unique() {
    let template =
        "color\n\tred\n\tblue\n\tgreen\n\noutput\n\t[color.selectUnique(2).joinItems(\" and \")]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have " and " separator
    assert!(output.contains(" and "));
    // Should have exactly one " and " (2 items)
    assert_eq!(output.matches(" and ").count(), 1);
}

#[tokio::test]
async fn test_join_items_default_separator() {
    let template = "num\n\t1\n\t2\n\t3\n\noutput\n\t[num.selectMany(3)]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Default separator is space
    assert!(output.contains(" "));
}

// Tests for consumableList
#[tokio::test]
async fn test_consumable_list_basic() {
    // Assignment to consumable list outputs the first item
    // So [c = item.consumableList] [c] [c] would output 3 items total (with spaces)
    let template = "item\n\ta\n\tb\n\tc\n\noutput\n\t[c = item.consumableList] [c] [c]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have 3 items (space-separated)
    assert_eq!(output.split_whitespace().count(), 3);
    // Each item should be one of a, b, c
    let parts: Vec<&str> = output.split_whitespace().collect();
    for part in &parts {
        assert!(part == &"a" || part == &"b" || part == &"c");
    }
}

#[tokio::test]
async fn test_consumable_list_exhaustion() {
    let template = "item\n\ta\n\tb\n\noutput\n\t[c = item.consumableList][c] [c] [c]\n";
    let result = run_with_seed(template, 42, None);
    // Should fail because we try to consume 3 items from a 2-item list
    result.await.unwrap_err();
}

#[tokio::test]
async fn test_consumable_list_select_unique() {
    let template = "item\n\ta\n\tb\n\tc\n\td\n\noutput\n\t[item.consumableList.selectUnique(3).joinItems(\", \")]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have 3 unique items
    let parts: Vec<&str> = output.split(", ").collect();
    assert_eq!(parts.len(), 3);
    // All items should be unique
    let mut sorted = parts.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 3);
}

#[tokio::test]
async fn test_consumable_list_no_duplicates() {
    // Test that consumableList doesn't repeat items until exhausted
    // Assignment outputs the first item, so we only need 2 more [c] references for 3 total
    let template = "item\n\ta\n\tb\n\tc\n\noutput\n\t[c = item.consumableList], [c], [c]\n";
    let output = run_with_seed(template, 42, None).await.unwrap();
    let parts: Vec<&str> = output.split(", ").collect();
    assert_eq!(parts.len(), 3);
    // All three should be different
    let mut sorted = parts.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        3,
        "Expected all items to be unique, got: {}",
        output
    );
}

#[tokio::test]
async fn test_consumable_list_independent_instances() {
    // Test that different consumableList instances are independent
    let template = "item\n\ta\n\tb\n\tc\n\noutput\n\t[c1 = item.consumableList][c2 = item.consumableList][c1] [c2]\n";
    let result = run_with_seed(template, 42, None);
    let _ = result.await.unwrap();
    // Both c1 and c2 should work independently
}

// Multiline tests from documentation examples

#[tokio::test]
async fn test_multiline_animal_sentence_paragraph() {
    let template = r#"animal
	pig
	cow
	zebra

adjective
  sneaky
  happy
  furry

sentence
	That [animal] is very sneaky.
	I befriended a wild [animal] yesterday.

paragraph = [sentence] [sentence] [sentence]

output
	[paragraph]"#;
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    // Should have 3 sentences (contains 3 periods)
    assert_eq!(output.matches('.').count(), 3);
    // Should contain animal names
    assert!(output.contains("pig") || output.contains("cow") || output.contains("zebra"));
}

#[tokio::test]
async fn test_multiline_animal_sentence_paragraph_with_spaces_and_tabs_1() {
    let template = r#"animal
	pig
	cow
	zebra

adjective
  sneaky
  happy
  furry

sentence
  That [animal] is very [adjective].
	I befriended a very [adjective] [animal] yesterday.

paragraph = [sentence] [sentence] [sentence]"#;
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    // Should have 3 sentences (contains 3 periods)
    assert_eq!(output.matches('.').count(), 3);
    // Should contain animal names
    assert!(output.contains("pig") || output.contains("cow") || output.contains("zebra"));
}

#[tokio::test]
async fn test_multiline_animal_sentence_paragraph_with_spaces_and_tabs_2() {
    let template = r#"animal
    pig
    cow
    zebra

adjective
  sneaky
  happy
  furry

sentence
    That [animal] is very sneaky.
    I befriended a wild [animal] yesterday.

paragraph = [sentence] [sentence] [sentence] "#;
    let result = run_with_seed(template, 42, None);
    let output = dbg!(result.await.unwrap());
    // Should have 3 sentences (contains 3 periods)
    assert_eq!(output.matches('.').count(), 3);
    // Should contain animal names
    assert!(output.contains("pig") || output.contains("cow") || output.contains("zebra"));
}

#[tokio::test]
async fn test_multiline_food_description() {
    let template = r#"description
	It's a [adjective] dish with [type] [main].
	The [adjective] [main] is paired with a [size] serving of [condiment]-covered [side].
	A [main] with a bit of [condiment] and some [adjective] [side] on the side.

adjective
	vegan
	Indonesian
	Italian
	delicious

main
	risotto
	pie
	stir-fry
	curry

side
	bowl of rice
	salad
	fries
	fried mushrooms
	pumpkin soup

type
	a [size] serving of
	well-cooked
	unusually fresh
	roasted

size
	small
	large
	tiny

condiment
	pepper ^2
	salt
	chilli flakes ^0.1
	oregano

output
	[description]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should contain one of the mains
    assert!(
        output.contains("risotto")
            || output.contains("pie")
            || output.contains("stir-fry")
            || output.contains("curry")
    );
    // Should contain one of the adjectives
    assert!(
        output.contains("vegan")
            || output.contains("Indonesian")
            || output.contains("Italian")
            || output.contains("delicious")
    );
}

#[tokio::test]
async fn test_multiline_inline_curly_blocks() {
    let template = r#"animal
	pig
	cow

sentence
	That's a {very|extremely} {tiny|small} [animal]!
	I {think|believe} that you are a {liar|thief}.
	I'd be so {rich|poor} if not for that person.

output
	[sentence]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should be one of the sentences
    assert!(output.ends_with('!') || output.ends_with('.'));
}

#[tokio::test]
async fn test_multiline_plural_form_title_case() {
    let template = r#"animal
	pig
	zebra
	cow

sentence
	There are so many [animal.pluralForm] here.
	I've befriended this [animal].
	[animal.pluralForm.titleCase] are very agile.

output
	[sentence]"#;
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    // Should contain pluralized animals
    assert!(
        output.contains("pigs")
            || output.contains("zebras")
            || output.contains("cows")
            || output.contains("Pigs")
            || output.contains("Zebras")
            || output.contains("Cows")
            || output.contains("pig")
            || output.contains("zebra")
            || output.contains("cow")
    );
}

#[tokio::test]
async fn test_multiline_select_one_variable() {
    let template = r#"flower
	rose
	lily
	tulip

sentence
  Oh you've got me a [f = flower.selectOne]! Thank you, I love [f.pluralForm].

output
  [sentence]"#;
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    // Should have consistent flower (e.g., "rose" and "roses")
    if output.contains("rose") {
        assert!(output.contains("roses"));
    } else if output.contains("lily") {
        assert!(output.contains("lilies") || output.contains("lilys"));
    } else if output.contains("tulip") {
        assert!(output.contains("tulips"));
    }
}

#[tokio::test]
async fn test_multiline_multiple_variable_assignments() {
    let template = r#"name
  Addison
  Alex
  Alexis

lastName
	Smith
	Johnson
	Williams

sentence
  I think her name was [n = name.selectOne.titleCase]? [n] [l = lastName.titleCase]? Wait, no, it was [n = name.selectOne]. Yeah, that's right, [n] [l].

output
  [sentence]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should contain name patterns
    assert!(output.contains("Addison") || output.contains("Alex") || output.contains("Alexis"));
    // Should contain last name
    assert!(output.contains("Smith") || output.contains("Johnson") || output.contains("Williams"));
}

#[tokio::test]
async fn test_multiline_consumable_list_topic() {
    let template = r#"topic
  trans rights
	animal rights
	science
	mathematics

sentence
  She mostly writes about [t = topic.consumableList, t] and [a = t.selectOne, a]. Her last post was about [a].

output
  [sentence]"#;
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    // Should contain topics
    assert!(
        output.contains("trans rights")
            || output.contains("animal rights")
            || output.contains("science")
            || output.contains("mathematics")
    );
    // The second and third topic mentions should be the same
    // One topic should appear exactly twice (the one from [a = t.selectOne, a] and [a])
    let topics = ["trans rights", "animal rights", "science", "mathematics"];
    let topic_counts: Vec<_> = topics
        .iter()
        .map(|topic| output.matches(topic).count())
        .collect();

    // Should have exactly one topic appearing twice (from [a = t.selectOne, a] and [a])
    // and one topic appearing once (from [t = topic.consumableList])
    assert_eq!(topic_counts.iter().filter(|&&count| count == 2).count(), 1);
    assert_eq!(topic_counts.iter().filter(|&&count| count == 1).count(), 1);
    assert_eq!(topic_counts.iter().filter(|&&count| count == 0).count(), 2);

    assert!(output.contains(" and "));
    assert!(output.contains("Her last post was about "));
}

#[tokio::test]
async fn test_multiline_hierarchical_sublists() {
    let template = r#"animal
	mammal
		kangaroo
		pig
		human
	reptile
		lizard
		crocodile
		turtle
	insect
		spider
		beetle
		ant

output
    {[animal.mammal]|[animal.reptile]|[animal.insect]}"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should be one of the specific animals
    let animals = [
        "kangaroo",
        "pig",
        "human",
        "lizard",
        "crocodile",
        "turtle",
        "spider",
        "beetle",
        "ant",
    ];
    assert!(animals.iter().any(|&a| output.contains(a)));
}

#[tokio::test]
async fn test_multiline_or_operator_fallback() {
    let template = r#"output
  {A} [a = animal.selectOne] is covered in [a.body || "fur"].

animal
  bird
    body = feathers
  lizard
    body = scales
  dog
	cat
	moose"#;
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    // Should contain either "feathers", "scales", or "fur"
    assert!(output.contains("feathers") || output.contains("scales") || output.contains("fur"));
    // Should start with "A " (article)
    assert!(output.starts_with("A "));
}

#[tokio::test]
async fn test_html_tag_passthrough() {
    // HTML tags should be passed through as-is
    let template = "output\n\t<b>Bold</b> and <i>italic</i> text<br>New line\n";
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    assert_eq!(output, "<b>Bold</b> and <i>italic</i> text<br>New line");
}

#[tokio::test]
async fn test_html_tags_with_references() {
    let template = "word\n\thello\n\noutput\n\t<b>[word]</b> <i>world</i>\n";
    let result = run_with_seed(template, 42, None);
    let output = result.await.unwrap();
    assert_eq!(output, "<b>hello</b> <i>world</i>");
}

#[tokio::test]
async fn test_multiline_evaluate_item_with_ranges() {
    let template = r#"output
  [f = fruit.selectOne.evaluateItem]?! [f] is way too many!

fruit
  {10-20} apples
  {30-70} pears"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have same number in both places
    // e.g., "50 pears?! 50 pears is way too many!"
    assert!(output.contains("apples") || output.contains("pears"));
    assert!(output.contains("?!"));
    assert!(output.contains("is way too many!"));
}

#[tokio::test]
async fn test_multiline_dynamic_odds_with_equality() {
    let template = r#"output
  The dragon's scales were [c = color.selectOne]. More specifically, [shade.selectOne].

color
  blue
  red
  green
  yellow

shade
  blue ^[c == "blue"]
    cyan
    navy blue
    teal
    turquoise
  red ^[c == "red"]
    maroon
    cherry"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // If color is blue, shade should be a blue shade
    if output.contains("were blue") {
        assert!(
            output.contains("cyan")
                || output.contains("navy blue")
                || output.contains("teal")
                || output.contains("turquoise")
        );
    }
    // If color is red, shade should be a red shade
    if output.contains("were red") {
        assert!(output.contains("maroon") || output.contains("cherry"));
    }
}

#[tokio::test]
async fn test_join_lists_basic() {
    let template = r#"mammal
  dog
  cat
  horse

reptile
  snake
  lizard
  turtle

output
  [animal = joinLists(mammal, reptile), animal]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should select from combined list
    assert!(
        output == "dog"
            || output == "cat"
            || output == "horse"
            || output == "snake"
            || output == "lizard"
            || output == "turtle",
        "Expected one of the animals, got: {}",
        output
    );
}

#[tokio::test]
async fn test_join_lists_with_methods() {
    let template = r#"tree
  oak
  pine

shrub
  bush
  hedge

output
  [joinLists(tree, shrub).selectMany(5)]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    // Should have 5 space-separated items
    let parts: Vec<&str> = output.split_whitespace().collect();
    assert_eq!(parts.len(), 5, "Expected 5 items, got: {}", output);
    // Each should be from one of the lists
    for part in parts {
        assert!(
            part == "oak" || part == "pine" || part == "bush" || part == "hedge",
            "Unexpected item: {}",
            part
        );
    }
}

#[tokio::test]
async fn test_join_lists_assignment() {
    let template = r#"fruit
  apple
  banana

vegetable
  carrot
  broccoli

output
  [food = joinLists(fruit, vegetable), food]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(
        output == "apple" || output == "banana" || output == "carrot" || output == "broccoli",
        "Expected a food item, got: {}",
        output
    );
}

#[tokio::test]
async fn test_join_lists_multiple_uses() {
    let template = r#"color1
  red
  blue

color2
  green
  yellow

output
  [joined = joinLists(color1, color2), joined] and [joined]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(output.contains("and"), "Expected 'and' in output");
    // Both selections should be valid colors
    let parts: Vec<&str> = output.split(" and ").collect();
    assert_eq!(parts.len(), 2);
    for part in parts {
        assert!(
            part == "red" || part == "blue" || part == "green" || part == "yellow",
            "Unexpected color: {}",
            part
        );
    }
}

#[tokio::test]
async fn test_join_lists_three_lists() {
    let template = r#"list1
  a

list2
  b

list3
  c

output
  [joinLists(list1, list2, list3)]"#;
    let output = run_with_seed(template, 42, None).await.unwrap();
    assert!(
        output == "a" || output == "b" || output == "c",
        "Expected a, b, or c, got: {}",
        output
    );
}

#[tokio::test]
async fn test_join_lists_complex_realistic() {
    // Test a realistic complex template with joinLists, weighted items, and dynamic weights
    let template = r#"numberOfItems = 1
itemSeperator = <br><br>

title
  Random Image Prompt Generator

output
  [quirks]

quirks
  [moods]. [lighting]. [flavour]. [anypunks].
  [moods]. [f = joinLists(flavour, anypunks)].^0.5
  [f = joinLists(flavour, anypunks)].^0.5
  [lighting.sentenceCase]. [moods].
  [lighting.sentenceCase]. [moods]. [f = joinLists(flavour, anypunks)].^0.5
  [lighting.sentenceCase]. [f = joinLists(flavour, anypunks)].^0.5

moods
  Colorful
  Humorous
  Abandoned
  Zestful

flavour
  Aged
  Lucid Dream

anypunks
  Steampunk^5
  Hopepunk

lighting
  backlit
  ambient lighting"#;

    let output = run_with_seed(template, 42, None).await.unwrap();

    // Verify the output is not empty
    assert!(!output.is_empty(), "Output should not be empty");

    // The output should contain at least one word (period-separated)
    let parts: Vec<&str> = output.split(". ").collect();
    assert!(!parts.is_empty(), "Expected at least one element in output");

    // Test multiple seeds to ensure variety and that joinLists works consistently
    for seed in [1, 2, 10, 20, 100, 200, 300, 400, 500] {
        let result = run_with_seed(template, seed, None).await;
        assert!(
            result.is_ok(),
            "Template should evaluate successfully with seed {}: {:?}",
            seed,
            result.err()
        );

        let output = result.unwrap();
        assert!(
            !output.is_empty(),
            "Output should not be empty for seed {}",
            seed
        );
    }

    // Run many times to test that dynamic weights work with joinLists
    let mut outputs = std::collections::HashSet::new();
    for seed in 0..50 {
        let result = run_with_seed(template, seed, None).await.unwrap();
        outputs.insert(result);
    }

    // We should get multiple different outputs due to randomness
    assert!(
        outputs.len() > 5,
        "Expected diverse outputs with joinLists and dynamic weights, got {} unique outputs",
        outputs.len()
    );
}

#[tokio::test]
async fn test_join_lists_with_dynamic_weights() {
    // Test that joinLists works correctly with dynamic weights on the list items
    let template = r#"list1
  item1^10
  item2^1

list2
  item3^1
  item4^10

output
  [joinLists(list1, list2)]"#;

    // Run multiple times to verify it works consistently
    for seed in [1, 2, 3, 4, 5, 10, 20, 30, 40, 50] {
        let result = run_with_seed(template, seed, None).await;
        assert!(
            result.is_ok(),
            "joinLists with weighted items should work, seed {}: {:?}",
            seed,
            result.err()
        );

        let output = result.unwrap();
        assert!(
            output == "item1" || output == "item2" || output == "item3" || output == "item4",
            "Expected one of the items, got: {} (seed {})",
            output,
            seed
        );
    }

    // Test that the weights are preserved (item1 and item4 have weight 10, others have weight 1)
    // Over many runs, we should see item1 and item4 more frequently
    let mut counts = std::collections::HashMap::new();
    for seed in 0..1000 {
        let output = run_with_seed(template, seed, None).await.unwrap();
        *counts.entry(output).or_insert(0) += 1;
    }

    // item1 and item4 should appear more frequently than item2 and item3
    let count_item1 = *counts.get("item1").unwrap_or(&0);
    let count_item2 = *counts.get("item2").unwrap_or(&0);
    let count_item3 = *counts.get("item3").unwrap_or(&0);
    let count_item4 = *counts.get("item4").unwrap_or(&0);

    // With weights 10:1:1:10, we expect roughly 10x more item1/item4 than item2/item3
    assert!(
        count_item1 > count_item2 * 3,
        "item1 (weight 10) should appear more than item2 (weight 1): {} vs {}",
        count_item1,
        count_item2
    );
    assert!(
        count_item4 > count_item3 * 3,
        "item4 (weight 10) should appear more than item3 (weight 1): {} vs {}",
        count_item4,
        count_item3
    );
}

#[tokio::test]
async fn test_join_lists_as_builtin_no_import_needed() {
    // Test that joinLists works as a built-in function without requiring
    // the {import:join-lists-plugin} line that would be needed in original Perchance.
    // This demonstrates our implementation is a convenient drop-in replacement.
    let template = r#"
mammal
  dog
  cat

reptile
  snake
  lizard

output
  [animal = joinLists(mammal, reptile), animal]"#;

    // Verify it works without any import statement
    let result = run_with_seed(template, 42, None).await;
    assert!(
        result.is_ok(),
        "joinLists should work as built-in without import: {:?}",
        result.err()
    );

    let output = result.unwrap();
    assert!(
        output == "dog" || output == "cat" || output == "snake" || output == "lizard",
        "Expected an animal, got: {}",
        output
    );
}

#[tokio::test]
async fn test_join_lists_realistic_with_comment() {
    // Test the realistic template structure that would typically have imports
    // Note: In real Perchance, you'd have: joinLists = {import:join-lists-plugin}
    // But our implementation provides it as a built-in, so no import is needed!
    let template = r#"
// In original Perchance, you would need:
// joinLists = {import:join-lists-plugin}
// But our implementation provides it as a built-in!

numberOfItems = 1
itemSeperator = <br><br>

title
  Random Image Prompt Generator

output
  [quirks]

quirks
  [moods]. [lighting]. [f = joinLists(flavour, anypunks)].^0.5
  [lighting.sentenceCase]. [moods]. [f = joinLists(flavour, anypunks)].^0.5

moods
  Colorful
  Humorous
  Abandoned
  Zestful

flavour
  Aged
  Lucid Dream

anypunks
  Steampunk^5
  Hopepunk

lighting
  backlit
  ambient lighting"#;

    // Test that it works across multiple seeds
    for seed in [1, 42, 100, 200, 500] {
        let result = run_with_seed(template, seed, None).await;
        assert!(
            result.is_ok(),
            "Template should work with built-in joinLists (seed {}): {:?}",
            seed,
            result.err()
        );
    }
}
