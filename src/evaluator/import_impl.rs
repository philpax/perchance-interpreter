//! Import-related implementation for the Evaluator
//!
//! This module contains the load_import method and related functionality
//! for handling generator imports in the Perchance interpreter.

use crate::compiler::CompiledProgram;
use crate::span::Span;
use rand::Rng;

use super::{EvalError, Evaluator};

impl<'a, R: Rng + Send> Evaluator<'a, R> {
    /// Load and compile an imported generator
    ///
    /// This method handles loading external generators by:
    /// 1. Checking the import cache for previously loaded generators
    /// 2. Using the loader to fetch the generator source code
    /// 3. Parsing and compiling the source into a CompiledProgram
    /// 4. Caching both the source and compiled program for future use
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the generator to load
    /// * `span` - The span information for error reporting
    ///
    /// # Returns
    ///
    /// * `Ok(&CompiledProgram)` - A reference to the cached compiled program
    /// * `Err(EvalError)` - An error if loading, parsing, or compilation fails
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - No loader is available for imports
    /// - The loader fails to fetch the generator source
    /// - The source fails to parse
    /// - The parsed program fails to compile
    pub async fn load_import(
        &mut self,
        name: &str,
        span: Span,
    ) -> Result<&CompiledProgram, EvalError> {
        // Check cache first
        if self.import_cache.contains_key(name) {
            return Ok(self.import_cache.get(name).unwrap());
        }

        // Check if loader is available
        let loader = self.loader.as_ref().ok_or_else(|| EvalError::ImportError {
            message: "No loader available for imports".to_string(),
            span,
        })?;

        // Load the generator source
        let source = loader
            .load(name)
            .await
            .map_err(|e| EvalError::ImportError {
                message: format!("Failed to load generator '{}': {}", name, e),
                span,
            })?;

        // Store the source for tracing
        self.import_sources.insert(name.to_string(), source.clone());

        // Parse and compile the generator
        let program = crate::parser::parse(&source).map_err(|e| EvalError::ImportError {
            message: format!("Failed to parse generator '{}': {}", name, e),
            span,
        })?;

        let compiled = crate::compiler::compile(&program).map_err(|e| EvalError::ImportError {
            message: format!("Failed to compile generator '{}': {}", name, e),
            span,
        })?;

        // Cache it
        self.import_cache.insert(name.to_string(), compiled);

        Ok(self.import_cache.get(name).unwrap())
    }
}
