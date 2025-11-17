/// Perchance Interpreter - A deterministic random text generator
///
/// This library implements an interpreter for the Perchance template language,
/// focused on core functionality with deterministic random generation.
///
/// # Example
///
/// ```
/// # tokio_test::block_on(async {
/// use perchance_interpreter::evaluate_with_seed;
///
/// let template = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a [animal].\n";
/// let result = evaluate_with_seed(template, 42).await.unwrap();
/// println!("{}", result);
/// # });
/// ```
pub mod ast;
pub mod compiler;
pub mod diagnostic;
pub mod evaluator;
pub mod loader;
pub mod parser;
pub mod span;

#[cfg(feature = "builtin-generators")]
pub mod builtin_generators;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

/// Re-export main types for convenience
pub use ast::Program;
pub use compiler::{CompileError, CompiledProgram};
pub use evaluator::EvalError;
pub use loader::GeneratorLoader;
pub use parser::ParseError;

/// Combined error type for the interpreter
#[derive(Debug)]
pub enum InterpreterError {
    Parse(ParseError),
    Compile(CompileError),
    Eval(EvalError),
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::Parse(e) => write!(f, "Parse error: {}", e),
            InterpreterError::Compile(e) => write!(f, "Compile error: {}", e),
            InterpreterError::Eval(e) => write!(f, "Evaluation error: {}", e),
        }
    }
}

impl std::error::Error for InterpreterError {}

impl From<ParseError> for InterpreterError {
    fn from(e: ParseError) -> Self {
        InterpreterError::Parse(e)
    }
}

impl From<CompileError> for InterpreterError {
    fn from(e: CompileError) -> Self {
        InterpreterError::Compile(e)
    }
}

impl From<EvalError> for InterpreterError {
    fn from(e: EvalError) -> Self {
        InterpreterError::Eval(e)
    }
}

/// Options for evaluating a compiled program
pub struct EvaluateOptions<R: Rng + Send> {
    /// Random number generator
    pub rng: R,
    /// Generator loader for imports (optional, defaults to BuiltinGeneratorsLoader if available)
    pub loader: Option<Arc<dyn GeneratorLoader>>,
}

impl<R: Rng + Send> EvaluateOptions<R> {
    /// Create new options with provided RNG
    pub fn new(rng: R) -> Self {
        EvaluateOptions { rng, loader: None }
    }

    /// Set the loader
    pub fn with_loader(mut self, loader: Arc<dyn GeneratorLoader>) -> Self {
        self.loader = Some(loader);
        self
    }
}

/// Parse a Perchance template into an AST
///
/// # Example
/// ```
/// use perchance_interpreter::parse;
///
/// let template = "output\n\thello\n";
/// let program = parse(template).unwrap();
/// ```
pub fn parse(input: &str) -> Result<Program, ParseError> {
    parser::parse(input)
}

/// Compile an AST into an evaluatable program
///
/// # Example
/// ```
/// use perchance_interpreter::{parse, compile};
///
/// let template = "output\n\thello\n";
/// let program = parse(template).unwrap();
/// let compiled = compile(&program).unwrap();
/// ```
pub fn compile(program: &Program) -> Result<CompiledProgram, CompileError> {
    compiler::compile(program)
}

/// Evaluate a compiled program with options
///
/// # Example
/// ```
/// # tokio_test::block_on(async {
/// use perchance_interpreter::{parse, compile, evaluate, EvaluateOptions};
/// use rand::SeedableRng;
/// use rand::rngs::StdRng;
///
/// let template = "output\n\thello\n";
/// let program = parse(template).unwrap();
/// let compiled = compile(&program).unwrap();
///
/// let rng = StdRng::seed_from_u64(42);
/// let options = EvaluateOptions::new(rng);
/// let result = evaluate(&compiled, options).await.unwrap();
/// # });
/// ```
pub async fn evaluate<R: Rng + Send>(
    program: &CompiledProgram,
    mut options: EvaluateOptions<R>,
) -> Result<String, EvalError> {
    // Get or create default loader
    let loader = if let Some(loader) = options.loader {
        Some(loader)
    } else {
        #[cfg(feature = "builtin-generators")]
        {
            Some(Arc::new(loader::BuiltinGeneratorsLoader::new()) as Arc<dyn GeneratorLoader>)
        }
        #[cfg(not(feature = "builtin-generators"))]
        {
            None
        }
    };

    // Create evaluator
    let mut evaluator = evaluator::Evaluator::new(program, &mut options.rng);
    if let Some(loader) = loader {
        evaluator = evaluator.with_loader(loader);
    }

    evaluator.evaluate().await
}

/// Parse, compile, and evaluate a template in one step
///
/// # Example
/// ```
/// # tokio_test::block_on(async {
/// use perchance_interpreter::{run, EvaluateOptions};
/// use rand::SeedableRng;
/// use rand::rngs::StdRng;
///
/// let template = "output\n\thello\n";
/// let rng = StdRng::seed_from_u64(42);
/// let options = EvaluateOptions::new(rng);
/// let result = run(template, options).await.unwrap();
/// # });
/// ```
pub async fn run<R: Rng + Send>(
    template: &str,
    options: EvaluateOptions<R>,
) -> Result<String, InterpreterError> {
    let program = parse(template)?;
    let compiled = compile(&program)?;
    let result = evaluate(&compiled, options).await?;
    Ok(result)
}

/// Parse, compile, and evaluate a template with a seed
///
/// This is a convenience function for deterministic output.
///
/// # Example
/// ```
/// # tokio_test::block_on(async {
/// use perchance_interpreter::run_with_seed;
///
/// let template = "output\n\thello\n";
/// let result = run_with_seed(template, 42, None).await.unwrap();
/// # });
/// ```
pub async fn run_with_seed(
    template: &str,
    seed: u64,
    loader: Option<Arc<dyn GeneratorLoader>>,
) -> Result<String, InterpreterError> {
    let rng = StdRng::seed_from_u64(seed);
    let mut options = EvaluateOptions::new(rng);
    if let Some(loader) = loader {
        options = options.with_loader(loader);
    }
    run(template, options).await
}

/// Get list of available builtin generators
///
/// Returns the names of all available builtin generators if the
/// `builtin-generators` feature is enabled. Useful for autocomplete
/// and discovering what generators can be imported.
///
/// # Example
/// ```
/// use perchance_interpreter::list_builtin_generators;
///
/// let generators = list_builtin_generators();
/// println!("Available generators: {:?}", generators);
/// ```
pub fn list_builtin_generators() -> Vec<String> {
    #[cfg(feature = "builtin-generators")]
    {
        let loader = loader::BuiltinGeneratorsLoader::new();
        loader.list_available()
    }
    #[cfg(not(feature = "builtin-generators"))]
    {
        Vec::new()
    }
}

// Deprecated functions - kept for backward compatibility
#[deprecated(since = "0.1.0", note = "Use `run_with_seed` instead")]
pub async fn evaluate_with_seed(template: &str, seed: u64) -> Result<String, InterpreterError> {
    run_with_seed(template, seed, None).await
}

#[deprecated(since = "0.1.0", note = "Use `parse` and `compile` instead")]
pub fn compile_template(template: &str) -> Result<CompiledProgram, InterpreterError> {
    let program = parse(template)?;
    let compiled = compile(&program)?;
    Ok(compiled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_with_seed() {
        let template = "output\n\tHello world!\n";
        let result = run_with_seed(template, 42, None).await;
        assert_eq!(result.unwrap(), "Hello world!");
    }

    #[tokio::test]
    async fn test_deterministic_output() {
        let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";

        let result1 = run_with_seed(template, 12345, None).await.unwrap();
        let result2 = run_with_seed(template, 12345, None).await.unwrap();

        assert_eq!(result1, result2);
    }

    #[tokio::test]
    async fn test_parse_compile_evaluate() {
        let template = "output\n\t{1-10}\n";
        let program = parse(template).unwrap();
        let compiled = compile(&program).unwrap();

        let rng = StdRng::seed_from_u64(999);
        let options = EvaluateOptions::new(rng);
        let result = evaluate(&compiled, options).await;

        let num: i32 = result.unwrap().parse().unwrap();
        assert!((1..=10).contains(&num));
    }

    #[tokio::test]
    async fn test_complex_template() {
        let template = "animal\n\tdog\n\tcat^2\n\tbird^0.5\n\ncolor\n\tred\n\tblue\n\tgreen\n\noutput\n\tI saw a {beautiful|pretty} [color] [animal].\n";

        let result = run_with_seed(template, 42, None).await;
        let output = result.unwrap();
        assert!(output.starts_with("I saw a"));
    }
}
