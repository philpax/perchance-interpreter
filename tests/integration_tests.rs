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
