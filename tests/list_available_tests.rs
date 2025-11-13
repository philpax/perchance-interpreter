use perchance_interpreter::list_builtin_generators;
use perchance_interpreter::loader::{ChainLoader, GeneratorLoader, InMemoryLoader};
use std::sync::Arc;

#[test]
fn test_list_builtin_generators() {
    let generators = list_builtin_generators();

    // Should have at least 38 generators (there are currently 38 builtin generators)
    assert!(
        generators.len() >= 38,
        "Expected at least 38 generators, got {}",
        generators.len()
    );

    // Check for some known generators
    assert!(generators.contains(&"animal".to_string()));
    assert!(generators.contains(&"noun".to_string()));
    assert!(generators.contains(&"color".to_string()));

    println!("Found {} builtin generators", generators.len());
    println!("First 10: {:?}", &generators[..10.min(generators.len())]);
}

#[test]
fn test_in_memory_loader_list_available() {
    let loader = InMemoryLoader::new();
    loader.add("test1", "output\n\tvalue1\n");
    loader.add("test2", "output\n\tvalue2\n");
    loader.add("test3", "output\n\tvalue3\n");

    let mut available = loader.list_available();
    available.sort();

    assert_eq!(available, vec!["test1", "test2", "test3"]);
}

#[test]
fn test_chain_loader_list_available() {
    let memory_loader = InMemoryLoader::new();
    memory_loader.add("custom1", "output\n\tvalue\n");
    memory_loader.add("custom2", "output\n\tvalue\n");

    #[cfg(feature = "builtin-generators")]
    {
        use perchance_interpreter::loader::BuiltinGeneratorsLoader;

        let builtin_loader = BuiltinGeneratorsLoader::new();

        let chain = ChainLoader::new()
            .with_loader(Arc::new(memory_loader))
            .with_loader(Arc::new(builtin_loader));

        let available = chain.list_available();

        // Should have custom generators plus builtin ones
        assert!(available.contains(&"custom1".to_string()));
        assert!(available.contains(&"custom2".to_string()));
        assert!(available.contains(&"animal".to_string()));
        assert!(available.contains(&"noun".to_string()));

        println!("Chain loader has {} total generators", available.len());
    }
}
