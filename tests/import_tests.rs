/// Tests for import/export functionality
use perchance_interpreter::loader::InMemoryLoader;
use perchance_interpreter::run_with_seed;
use std::sync::Arc;

#[tokio::test]
async fn test_basic_import_inline() {
    // Create a loader with a simple generator
    let loader = InMemoryLoader::new();
    loader.add("nouns", "noun\n\tdog\n\tcat\n\tbird\n\noutput\n\t[noun]\n");

    // Create a generator that imports the nouns
    let template = "output\n\tI saw a {import:nouns}.\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    assert!(
        result == "I saw a dog." || result == "I saw a cat." || result == "I saw a bird.",
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_import_with_assignment() {
    // Create a loader with a simple generator
    let loader = InMemoryLoader::new();
    loader.add(
        "animals",
        "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n",
    );

    // Create a generator that imports and assigns
    let template = "myAnimals = {import:animals}\n\noutput\n\t[myAnimals]\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    assert!(
        result == "dog" || result == "cat" || result == "bird",
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_import_with_property_access() {
    // Create a loader with a generator that has multiple lists
    let loader = InMemoryLoader::new();
    loader.add(
        "creatures",
        "animal\n\tdog\n\tcat\n\ncolor\n\tred\n\tblue\n\noutput\n\t[color] [animal]\n",
    );

    // Access specific properties from the imported generator
    let template = "gen = {import:creatures}\n\noutput\n\t[gen.animal] is [gen.color]\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    // Should access individual lists
    assert!(result.contains(" is "), "Got: {}", result);
}

#[tokio::test]
async fn test_import_with_output_property() {
    // Create a loader with a generator that has $output
    let loader = InMemoryLoader::new();
    loader.add(
        "greetings",
        "greeting\n\thello\n\thi\n\nname\n\tworld\n\tfriend\n\n$output = [greeting] [name]\n",
    );

    // Import should respect $output
    let template = "output\n\t{import:greetings}!\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    assert!(
        result == "hello world!"
            || result == "hello friend!"
            || result == "hi world!"
            || result == "hi friend!",
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_import_default_export() {
    // Create a loader with a generator that doesn't have explicit $output
    let loader = InMemoryLoader::new();
    loader.add("colors", "color\n\tred\n\tblue\n\tgreen\n");

    // Should export the "output" list by default
    let template = "output\n\tMy favorite color is {import:colors}.\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    assert!(
        result.contains("red") || result.contains("blue") || result.contains("green"),
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_multiple_imports() {
    // Create a loader with multiple generators
    let loader = InMemoryLoader::new();
    loader.add("adjectives", "adj\n\tbig\n\tsmall\n\noutput\n\t[adj]\n");
    loader.add("nouns", "noun\n\tdog\n\tcat\n\noutput\n\t[noun]\n");

    // Use both imports in one template
    let template = "output\n\tA {import:adjectives} {import:nouns}.\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    assert!(
        result.starts_with("A ") && result.ends_with("."),
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_import_not_found() {
    // Create an empty loader
    let loader = InMemoryLoader::new();

    // Try to import a non-existent generator
    let template = "output\n\t{import:nonexistent}\n";
    let _err = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap_err();
}

#[tokio::test]
async fn test_import_without_loader() {
    // Try to import without a loader
    let template = "output\n\t{import:something}\n";
    let _err = run_with_seed(template, 42, None).await.unwrap_err();
}

#[tokio::test]
async fn test_import_caching() {
    // Create a loader with a generator
    let loader = InMemoryLoader::new();
    loader.add("cached", "item\n\tvalue\n\noutput\n\t[item]\n");

    // Import the same generator multiple times
    let template = "output\n\t{import:cached} {import:cached}\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    // Should work and be cached (no need to load twice)
    assert_eq!(result, "value value");
}

#[tokio::test]
async fn test_import_with_weights() {
    // Create a loader with a weighted generator
    let loader = InMemoryLoader::new();
    loader.add(
        "weighted",
        "choice\n\trare^0.1\n\tcommon^10\n\noutput\n\t[choice]\n",
    );

    // Import should respect weights
    let template = "output\n\t{import:weighted}\n";

    // Run multiple times and ensure we get results (weights are working)
    let mut got_common = false;
    for seed in 0..100 {
        let result = run_with_seed(template, seed, Some(Arc::new(loader.clone())))
            .await
            .unwrap();
        if result == "common" {
            got_common = true;
            break;
        }
    }

    assert!(
        got_common,
        "Should get 'common' at least once with proper weights"
    );
}

#[tokio::test]
async fn test_import_with_sublists() {
    // Create a loader with a generator that has sublists
    let loader = InMemoryLoader::new();
    loader.add(
        "creature",
        "animal\n\tdog\n\t\tbreed\n\t\t\tlabrador\n\t\t\tpoodle\n\tcat\n\t\tbreed\n\t\t\tsiamese\n\t\t\tpersian\n\noutput\n\t[animal]\n",
    );

    // Import should work with sublists
    let template = "output\n\t{import:creature}\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    // With sublists, any of the animals or breeds might be selected
    let valid_results = ["dog", "cat", "labrador", "poodle", "siamese", "persian"];
    assert!(valid_results.contains(&result.as_str()), "Got: {}", result);
}

#[tokio::test]
async fn test_complex_import_scenario() {
    // Test a more complex scenario with nested imports and property access
    let loader = InMemoryLoader::new();
    loader.add(
        "base",
        "prefix\n\tMr.\n\tMs.\n\nname\n\tSmith\n\tJones\n\noutput\n\t[prefix] [name]\n",
    );

    let template = "person = {import:base}\ngreeting = hello\n\noutput\n\t[greeting] [person]!\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader)))
        .await
        .unwrap();

    assert!(result.starts_with("hello "), "Got: {}", result);
    assert!(result.ends_with("!"), "Got: {}", result);
}

#[tokio::test]
async fn test_import_parser_syntax() {
    // Test that various import syntaxes parse correctly
    let loader = InMemoryLoader::new();
    loader.add("test", "output\n\tvalue\n");

    let templates = vec![
        "{import:test}",              // Inline import
        " {import:test} ",            // With spaces
        "{import:test}{import:test}", // Multiple inline imports
    ];

    for template_str in templates {
        let template = format!("output\n\t{}\n", template_str);
        run_with_seed(&template, 42, Some(Arc::new(loader.clone())))
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn test_import_with_consumable_list() {
    // Test that we can use .consumableList on an imported generator
    let loader = InMemoryLoader::new();
    loader.add("color", "color\n\tred\n\tblue\n\tgreen\n");

    let template = "color = {import:color}\n\ntest\n\t[c = color.consumableList]\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader))).await;

    // This should work - creating a consumable list from imported generator
    let output = result.unwrap();
    assert!(output == "red" || output == "blue" || output == "green");
}

#[tokio::test]
async fn test_import_consumable_list_multiple_selections() {
    // Test that consumable lists work correctly with multiple selections from imported generator
    let loader = InMemoryLoader::new();
    loader.add("item", "item\n\ta\n\tb\n\tc\n");

    let template = "items = {import:item}\n\ntest\n\t[c = items.consumableList] [c] [c]\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader))).await;

    let output = result.unwrap();
    // Assignment outputs first item, then we select 2 more = 3 items total
    let parts: Vec<&str> = output.split_whitespace().collect();
    assert_eq!(parts.len(), 3, "Expected 3 parts, got: {}", output);

    // All items should be unique
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
async fn test_import_consumable_list_with_output_assignment() {
    // Test that consumableList works with imported generators that use $output = [listname]
    let loader = InMemoryLoader::new();
    loader.add(
        "color",
        "color\n\tred\n\tblue\n\tgreen\n\n$output = [color]\n",
    );

    let template = "colors = {import:color}\n\ntest\n\t[c = colors.consumableList] [c] [c]\n";
    let result = run_with_seed(template, 42, Some(Arc::new(loader))).await;

    let output = result.unwrap();
    // Should have 3 unique items
    let parts: Vec<&str> = output.split_whitespace().collect();
    assert_eq!(parts.len(), 3, "Expected 3 parts, got: {}", output);

    // All items should be unique
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
