use perchance_interpreter::loader::InMemoryLoader;
use perchance_interpreter::run_with_seed;
use std::sync::Arc;

/// Test that top-level $output takes priority over output list
#[tokio::test]
async fn test_dollar_output_priority() {
    let template = r#"description
	item1
	item2
	$output = <p>nested output</p>

$output = [god]

output
	should not be selected

god
	Zeus
	Hera
	Poseidon
"#;

    let result = run_with_seed(template, 42, None).await.unwrap();

    // Should get output from $output = [god], not from the output list or description's $output
    assert!(
        result == "Zeus" || result == "Hera" || result == "Poseidon",
        "Expected god name, got: {}",
        result
    );
    assert_ne!(result, "should not be selected");
    assert_ne!(result, "<p>nested output</p>");
}

/// Test greek-god pattern with import
#[tokio::test]
async fn test_greek_god_pattern_with_import() {
    let loader = InMemoryLoader::new();

    // Add a simplified version of the greek-god generator
    loader.add(
        "greek-god",
        r#"description
	Some description text
	$output = <p>[this.joinItems("</p><p>")]</p>

$output = [god]

output
	[c=god.selectOne]<p style="opacity:0.5;">wikipedia link</p>

god
	Aphrodite
	Apollo
	Ares
"#,
    );

    // Use the imported generator
    let template = "output\n\tThe god is {import:greek-god}.\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    // Should get just the god name, not the HTML output or the wikipedia link
    assert!(
        result.contains("Aphrodite") || result.contains("Apollo") || result.contains("Ares"),
        "Expected god name in output, got: {}",
        result
    );
    assert!(!result.contains("<p"), "Should not contain HTML tags");
    assert!(
        !result.contains("wikipedia"),
        "Should not contain wikipedia link"
    );
}

/// Test that $output property within a list is used when referencing that list
#[tokio::test]
async fn test_nested_dollar_output_isolation() {
    let template = r#"animal
	dog
	cat
	$output = nested animal

output
	I saw a [animal].
"#;

    let result = run_with_seed(template, 42, None).await.unwrap();

    // When referencing [animal], it should use the $output property of that list
    assert_eq!(result, "I saw a nested animal.");
}

/// Test output priority: $output > output > last list
#[tokio::test]
async fn test_output_priority_order() {
    // Test 1: $output takes priority
    let template1 = r#"$output = first priority
output
	second priority
last
	third priority
"#;
    let result1 = run_with_seed(template1, 42, None).await.unwrap();
    assert_eq!(result1, "first priority");

    // Test 2: output takes priority when no $output
    let template2 = r#"output
	second priority
last
	third priority
"#;
    let result2 = run_with_seed(template2, 42, None).await.unwrap();
    assert_eq!(result2, "second priority");

    // Test 3: last list when no output or $output
    let template3 = r#"first
	not this
last
	third priority
"#;
    let result3 = run_with_seed(template3, 42, None).await.unwrap();
    assert_eq!(result3, "third priority");
}
