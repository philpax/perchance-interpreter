/// Perchance Interpreter - A deterministic random text generator
///
/// This library implements an interpreter for the Perchance template language,
/// focused on core functionality with deterministic random generation.
///
/// # Example
///
/// ```
/// use perchance_interpreter::evaluate_with_seed;
///
/// let template = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a [animal].\n";
/// let result = evaluate_with_seed(template, 42).unwrap();
/// println!("{}", result);
/// ```
pub mod ast;
pub mod compiler;
pub mod evaluator;
pub mod parser;

use rand::rngs::StdRng;
use rand::SeedableRng;

/// Re-export main types for convenience
pub use ast::Program;
pub use compiler::{CompileError, CompiledProgram};
pub use evaluator::EvalError;
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

/// Parse a Perchance template into an AST
pub fn parse(input: &str) -> Result<Program, ParseError> {
    parser::parse(input)
}

/// Compile an AST into an evaluatable program
pub fn compile(program: &Program) -> Result<CompiledProgram, CompileError> {
    compiler::compile(program)
}

/// Evaluate a compiled program with a provided RNG
pub fn evaluate<R: rand::Rng>(program: &CompiledProgram, rng: &mut R) -> Result<String, EvalError> {
    evaluator::evaluate(program, rng)
}

/// Convenience function: evaluate a template with a seed value
///
/// This function parses, compiles, and evaluates a Perchance template
/// using a seeded RNG for deterministic output.
///
/// # Arguments
///
/// * `template` - The Perchance template string
/// * `seed` - A seed value for the random number generator
///
/// # Example
///
/// ```
/// use perchance_interpreter::evaluate_with_seed;
///
/// let template = "output\n\tHello {world|universe}!\n";
/// let result = evaluate_with_seed(template, 12345).unwrap();
/// println!("{}", result);
/// ```
pub fn evaluate_with_seed(template: &str, seed: u64) -> Result<String, InterpreterError> {
    let program = parse(template)?;
    let compiled = compile(&program)?;
    let mut rng = StdRng::seed_from_u64(seed);
    let result = evaluate(&compiled, &mut rng)?;
    Ok(result)
}

/// Compile a template for repeated evaluation
///
/// This function parses and compiles a template, returning a compiled program
/// that can be evaluated multiple times with different RNGs.
///
/// # Example
///
/// ```
/// use perchance_interpreter::{compile_template, evaluate};
/// use rand::SeedableRng;
/// use rand::rngs::StdRng;
///
/// let template = "output\n\t{1-100}\n";
/// let compiled = compile_template(template).unwrap();
///
/// // Evaluate multiple times with different seeds
/// let mut rng1 = StdRng::seed_from_u64(1);
/// let result1 = evaluate(&compiled, &mut rng1).unwrap();
///
/// let mut rng2 = StdRng::seed_from_u64(2);
/// let result2 = evaluate(&compiled, &mut rng2).unwrap();
/// ```
pub fn compile_template(template: &str) -> Result<CompiledProgram, InterpreterError> {
    let program = parse(template)?;
    let compiled = compile(&program)?;
    Ok(compiled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_with_seed() {
        let template = "output\n\tHello world!\n";
        let result = evaluate_with_seed(template, 42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world!");
    }

    #[test]
    fn test_deterministic_output() {
        let template = "animal\n\tdog\n\tcat\n\tbird\n\noutput\n\t[animal]\n";

        let result1 = evaluate_with_seed(template, 12345).unwrap();
        let result2 = evaluate_with_seed(template, 12345).unwrap();

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_compile_and_evaluate() {
        let template = "output\n\t{1-10}\n";
        let compiled = compile_template(template).unwrap();

        let mut rng = StdRng::seed_from_u64(999);
        let result = evaluate(&compiled, &mut rng);
        assert!(result.is_ok());

        let num: i32 = result.unwrap().parse().unwrap();
        assert!((1..=10).contains(&num));
    }

    #[test]
    fn test_complex_template() {
        let template = "animal\n\tdog\n\tcat^2\n\tbird^0.5\n\ncolor\n\tred\n\tblue\n\tgreen\n\noutput\n\tI saw a {beautiful|pretty} [color] [animal].\n";

        let result = evaluate_with_seed(template, 42);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.starts_with("I saw a"));
    }
}
