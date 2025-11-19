/// Evaluator module - executes compiled programs with RNG support
// Sub-modules
mod error;
mod grammar;
mod value;

// Implementation modules
mod content_impl;
mod evaluate_impl;
mod expression_impl;
mod helpers_impl;
mod import_impl;
mod list_impl;
mod property_impl;
mod trace_impl;

// Public exports
pub use error::EvalError;

// Internal imports
use value::{ConsumableListState, Value};

use crate::compiler::*;
use crate::loader::GeneratorLoader;
use crate::trace::TraceNode;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

/// Evaluator executes compiled programs with RNG support
pub struct Evaluator<'a, R: Rng> {
    program: &'a CompiledProgram,
    rng: &'a mut R,
    variables: HashMap<String, Value>,
    last_number: Option<i64>, // Track last number for {s} pluralization
    current_item: Option<CompiledItem>, // Track current item for `this` keyword
    dynamic_properties: HashMap<String, Value>, // Track dynamic properties assigned via `this.property = value`
    consumable_lists: HashMap<String, ConsumableListState>, // Track consumable lists
    consumable_list_counter: usize,             // Counter for generating unique IDs
    loader: Option<Arc<dyn GeneratorLoader>>,   // Generator loader for imports
    import_cache: HashMap<String, CompiledProgram>, // Cache for imported generators
    import_sources: HashMap<String, String>,    // Cache for import source templates
    trace_enabled: bool,                        // Whether to collect trace information
    trace_stack: Vec<TraceNode>,                // Stack of trace nodes being built
    current_source_template: Option<String>,    // Current source template for tracing
    current_generator_name: Option<String>,     // Current generator name for tracing
}

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    /// Create a new evaluator
    pub fn new(program: &'a CompiledProgram, rng: &'a mut R) -> Self {
        Evaluator {
            program,
            rng,
            variables: HashMap::new(),
            last_number: None,
            current_item: None,
            dynamic_properties: HashMap::new(),
            consumable_lists: HashMap::new(),
            consumable_list_counter: 0,
            loader: None,
            import_cache: HashMap::new(),
            import_sources: HashMap::new(),
            trace_enabled: false,
            trace_stack: Vec::new(),
            current_source_template: None,
            current_generator_name: None,
        }
    }

    /// Enable tracing for this evaluator
    pub fn with_tracing(mut self) -> Self {
        self.trace_enabled = true;
        self
    }

    /// Set the current source template and generator name for tracing
    pub fn with_source(mut self, template: String, name: String) -> Self {
        self.current_source_template = Some(template);
        self.current_generator_name = Some(name);
        self
    }

    /// Set the generator loader for handling imports
    pub fn with_loader(mut self, loader: Arc<dyn GeneratorLoader>) -> Self {
        self.loader = Some(loader);
        self
    }
}

/// Public API for evaluating a compiled program
pub async fn evaluate<R: Rng + Send>(
    program: &CompiledProgram,
    rng: &mut R,
) -> Result<String, EvalError> {
    let mut evaluator = Evaluator::new(program, rng);
    evaluator.evaluate().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::parser::parse;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[tokio::test]
    async fn test_simple_evaluation() {
        let input = "output\n\thello world\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        assert_eq!(result.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn test_list_reference() {
        let input = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a [animal].\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        let output = result.unwrap();
        assert!(output == "I saw a dog." || output == "I saw a cat.");
    }

    #[tokio::test]
    async fn test_deterministic() {
        let input = "animal\n\tdog\n\tcat\n\noutput\n\t[animal]\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();

        let mut rng1 = StdRng::seed_from_u64(12345);
        let result1 = evaluate(&compiled, &mut rng1).await.unwrap();

        let mut rng2 = StdRng::seed_from_u64(12345);
        let result2 = evaluate(&compiled, &mut rng2).await.unwrap();

        assert_eq!(result1, result2);
    }

    #[tokio::test]
    async fn test_inline_list() {
        let input = "output\n\t{big|small}\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        let output = result.unwrap();
        assert!(output == "big" || output == "small");
    }

    #[tokio::test]
    async fn test_number_range() {
        let input = "output\n\t{1-6}\n";
        let program = parse(input).unwrap();
        let compiled = compile(&program).unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let result = evaluate(&compiled, &mut rng).await;
        let output = result.unwrap();
        let num: i32 = output.parse().unwrap();
        assert!((1..=6).contains(&num));
    }
}
