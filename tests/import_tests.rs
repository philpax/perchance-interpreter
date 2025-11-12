/// Tests for import/export functionality
use perchance_interpreter::loader::InMemoryLoader;
use perchance_interpreter::{compile, evaluate, parse};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::sync::Arc;

#[tokio::test]
async fn test_basic_import_inline() {
    // Create a loader with a simple generator
    let loader = InMemoryLoader::new();
    loader.add(
        "nouns",
        "noun\n\tdog\n\tcat\n\tbird\n\noutput\n\t[noun]\n",
    );

    // Create a generator that imports the nouns
    let template = "output\n\tI saw a {import:nouns}.\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
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
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
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
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
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
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
    assert!(
        result == "hello world!" || result == "hello friend!" ||
        result == "hi world!" || result == "hi friend!",
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_import_default_export() {
    // Create a loader with a generator without $output (default export)
    let loader = InMemoryLoader::new();
    loader.add("colors", "color\n\tred\n\tblue\n\tgreen\n");

    // Should still be able to import and use it
    let template = "output\n\tThe color is {import:colors}.\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
    assert!(
        result == "The color is red." || result == "The color is blue." || result == "The color is green.",
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_multiple_imports() {
    // Create a loader with multiple generators
    let loader = InMemoryLoader::new();
    loader.add("animals", "animal\n\tdog\n\tcat\n\noutput\n\t[animal]\n");
    loader.add("colors", "color\n\tred\n\tblue\n\noutput\n\t[color]\n");

    // Import multiple generators
    let template = "output\n\t{import:colors} {import:animals}\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
    // Should have both a color and an animal
    assert!(result.contains(" "), "Got: {}", result);
}

#[tokio::test]
async fn test_import_not_found() {
    // Create an empty loader
    let loader = InMemoryLoader::new();

    // Try to import a non-existent generator
    let template = "output\n\t{import:nonexistent}\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Import error"));
}

#[tokio::test]
async fn test_import_without_loader() {
    // Try to use import without setting a loader
    let template = "output\n\t{import:test}\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng);
    // Note: not calling with_loader()

    let result = evaluator.evaluate();
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No loader available"));
}

#[tokio::test]
async fn test_import_caching() {
    // Create a loader with a generator
    let loader = InMemoryLoader::new();
    loader.add("test", "output\n\thello\n");

    // Import the same generator twice
    let template = "output\n\t{import:test} {import:test}\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
    assert_eq!(result, "hello hello");
}

#[tokio::test]
async fn test_import_with_weights() {
    // Create a loader with a weighted generator
    let loader = InMemoryLoader::new();
    loader.add(
        "weighted",
        "item\n\trare^0.1\n\tcommon^10\n\noutput\n\t[item]\n",
    );

    // Import and use multiple times to test weights
    let template = "output\n\t{import:weighted}\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    // Run multiple times and check that we get mostly "common"
    let mut common_count = 0;
    let mut rare_count = 0;

    for seed in 0..100 {
        let mut rng = StdRng::seed_from_u64(seed);
        let loader_clone = InMemoryLoader::new();
        loader_clone.add(
            "weighted",
            "item\n\trare^0.1\n\tcommon^10\n\noutput\n\t[item]\n",
        );
        let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
            .with_loader(Arc::new(loader_clone));

        let result = evaluator.evaluate().unwrap();
        if result == "common" {
            common_count += 1;
        } else if result == "rare" {
            rare_count += 1;
        }
    }

    // Common should be much more frequent than rare
    assert!(
        common_count > rare_count * 5,
        "common: {}, rare: {}",
        common_count,
        rare_count
    );
}

#[tokio::test]
async fn test_import_with_sublists() {
    // Create a loader with a generator that has sublists
    let loader = InMemoryLoader::new();
    loader.add(
        "creatures",
        "creature\n\tdog\n\t\tcolor\n\t\t\tbrown\n\t\t\tblack\n\tcat\n\t\tcolor\n\t\t\twhite\n\t\t\torange\n\noutput\n\t[creature]\n",
    );

    let template = "output\n\t{import:creatures}\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
    assert!(
        result == "brown" || result == "black" || result == "white" || result == "orange",
        "Got: {}",
        result
    );
}

#[tokio::test]
async fn test_complex_import_scenario() {
    // Create a complex scenario with multiple imports and property access
    let loader = InMemoryLoader::new();

    // Base components generator
    loader.add(
        "adjectives",
        "adjective\n\tbig\n\tsmall\n\noutput\n\t[adjective]\n",
    );

    // Animal generator that imports adjectives
    loader.add(
        "animals",
        "animal\n\tdog\n\tcat\n\noutput\n\t{import:adjectives} [animal]\n",
    );

    // Main generator that imports animals
    let template = "output\n\tI saw a {import:animals}.\n";
    let program = parse(template).unwrap();
    let compiled = compile(&program).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let mut evaluator = perchance_interpreter::evaluator::Evaluator::new(&compiled, &mut rng)
        .with_loader(Arc::new(loader));

    let result = evaluator.evaluate().unwrap();
    assert!(result.starts_with("I saw a "), "Got: {}", result);
    assert!(result.ends_with('.'), "Got: {}", result);
}

#[tokio::test]
async fn test_import_parser_syntax() {
    // Test that the parser correctly handles import syntax
    let template = "myGen = {import:test-generator}\n\noutput\n\t[myGen]\n";
    let program = parse(template);
    assert!(program.is_ok(), "Parser should handle import syntax");

    let template2 = "output\n\tInline: {import:inline-test}\n";
    let program2 = parse(template2);
    assert!(program2.is_ok(), "Parser should handle inline imports");
}
