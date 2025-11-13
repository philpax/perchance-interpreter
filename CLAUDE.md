# CLAUDE.md - Perchance Interpreter Development Guide

## Project Overview

This is a Rust-based interpreter for the Perchance template language, a random text generator system. The interpreter parses Perchance templates, compiles them into an efficient representation, and evaluates them with deterministic random generation.

## Architecture

The project follows a three-stage pipeline:

```
Template (String) → Parser → AST → Compiler → CompiledProgram → Evaluator → Output (String)
```

### Core Modules

1. **`ast.rs`** - Abstract Syntax Tree definitions
   - Defines all expression types, lists, items, and content parts
   - Core types: `Program`, `List`, `Item`, `Expression`, `ContentPart`

2. **`parser.rs`** - Template parsing
   - Converts raw template strings into AST
   - Handles indentation-based list structure
   - Parses inline expressions like `{import:...}`, `[list]`, etc.

3. **`compiler.rs`** - AST compilation
   - Transforms AST into optimized `CompiledProgram`
   - Pre-calculates list weights for efficient selection
   - Validates list references

4. **`evaluator.rs`** - **Fully Async** evaluation engine
   - Evaluates compiled programs with RNG support
   - Handles variable assignment, property access, method calls
   - Manages import/export functionality
   - **Note**: All evaluation methods are async due to import support

5. **`loader.rs`** - Generator loading system
   - Async trait `GeneratorLoader` for fetching external generators
   - `FolderLoader`: Loads from filesystem directory
   - `InMemoryLoader`: Stores generators in memory (used for tests)

6. **`lib.rs`** - Public API
   - Exposes convenient functions: `parse()`, `compile()`, `evaluate()`, `evaluate_with_seed()`
   - All evaluation functions are async

## Key Design Decisions

### Async Architecture

The entire evaluator is async to support importing external generators. This means:
- All `evaluate_*` methods are `async fn`
- Use `.await` when calling any evaluation function
- Tests must be `#[tokio::test]` async tests
- The `async-recursion` crate handles recursive async methods

### Import/Export System

- `{import:generator-name}` loads and evaluates external generators
- Generators are cached in `import_cache` to avoid reloading
- Property access like `gen.animal` delegates to imported generator's lists
- `$output` property controls what a list exports

## Development Workflow

### Before You Start

Always run these commands before committing:

```bash
# 1. Run tests
cargo test

# 2. Run clippy (linter)
cargo clippy

# 3. Fix clippy issues automatically
cargo clippy --fix --allow-dirty --allow-staged

# 4. Format code
cargo fmt

# 5. Check everything passes
cargo test && cargo clippy
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests for specific module
cargo test --test import_tests
```

**Important Testing Guidelines:**

- **ALWAYS use `.unwrap()` or `.unwrap_err()` instead of asserting on `.is_ok()` or `.is_err()`**
  - ❌ Bad: `assert!(result.is_ok(), "Should work");`
  - ✅ Good: `let output = result.unwrap();`
  - **Why?** Unwrapping provides the actual error message in the panic, while `is_ok()` assertions only give you a boolean failure without context.

- For error cases, use `.unwrap_err()`:
  - ❌ Bad: `assert!(result.is_err(), "Should fail");`
  - ✅ Good: `let _err = result.unwrap_err();`

- Example of good test error handling:
  ```rust
  #[tokio::test]
  async fn test_something() {
      let template = "...";
      // This will show the actual error if it fails
      let output = run_with_seed(template, 42, None).await.unwrap();
      assert_eq!(output, "expected");
  }

  #[tokio::test]
  async fn test_error_case() {
      let template = "...invalid...";
      // This will show the actual success value if it unexpectedly passes
      let _err = run_with_seed(template, 42, None).await.unwrap_err();
  }
  ```

### Clippy (Linter)

```bash
# Check for linting issues
cargo clippy

# Auto-fix issues
cargo clippy --fix --allow-dirty --allow-staged

# Strict mode
cargo clippy -- -W clippy::pedantic
```

### Formatting

```bash
# Format all code
cargo fmt

# Check if code is formatted (CI)
cargo fmt -- --check
```

## Common Patterns

### Adding New Expression Types

1. Add variant to `Expression` enum in `ast.rs`
2. Update parser in `parser.rs` to recognize the syntax
3. Add evaluation logic in `evaluator.rs`:
   - `evaluate_expression()` - for string output
   - `evaluate_to_value()` - for value output (used in property access)

### Adding New Methods

1. Add method name to `is_grammar_method()` if applicable
2. Add case in `call_method_value()` in `evaluator.rs`
3. Handle all `Value` variants (List, ListInstance, Text, etc.)

### Working with Imports

When implementing features that interact with imports:
- Remember imports are async - use `.await`
- Check if caching is working in `load_import()`
- Test with `InMemoryLoader` for unit tests
- Consider nested imports (imports within imports)

## Debugging Tips

### Parser Issues

- Check `tests/integration_tests.rs` for parsing examples
- Use `parse(template)` and print the AST with `{:#?}`
- Common issues: indentation, whitespace, special characters

### Evaluator Issues

- Check variable scope and lifetime
- Verify RNG seed for deterministic output
- Use `evaluate_with_seed()` for reproducible tests
- Remember all evaluation is async now

### Import Issues

- Check if `loader` is set on the evaluator
- Verify generator is in the cache
- Look for borrowing issues with `import_cache`
- Use `InMemoryLoader` for isolated tests

## Testing Strategy

### Unit Tests

- Each module has `#[cfg(test)]` section
- Test individual functions in isolation
- Use `#[tokio::test]` for async tests

### Integration Tests

- `tests/integration_tests.rs` - comprehensive template tests
- `tests/import_tests.rs` - import/export functionality
- Test realistic templates end-to-end

### Doctests

- Examples in doc comments are tested
- Use `# tokio_test::block_on(async { ... })` for async examples
- Keep examples simple and self-contained

## CI/CD Reminders

Before pushing:
```bash
cargo fmt && cargo clippy --fix --allow-dirty && cargo test
```

If tests fail:
1. Read the error message carefully
2. Check if it's an async issue (missing `.await`)
3. Verify RNG seed for deterministic tests
4. Check import loader is configured

## Async Best Practices

- **Always** mark functions as `async` if they call async functions
- Use `#[async_recursion]` for recursive async functions
- Don't forget `.await` on async calls
- Test async code with `#[tokio::test]`
- Use `tokio_test::block_on` for doctests

## Common Gotchas

1. **Forgetting `.await`** - Compiler error about future not being awaited
2. **Missing `Send` bound** - Add `R: Rng + Send` for generic RNG types
3. **Borrow checker with imports** - Clone data before crossing async boundaries
4. **Doctest async** - Wrap in `tokio_test::block_on(async { })`
5. **Method exhaustiveness** - All `Value` variants must be handled in match statements

## Resources

- Perchance docs: https://perchance.org/
- Rust async book: https://rust-lang.github.io/async-book/
- Tokio tutorial: https://tokio.rs/tokio/tutorial

## Quick Reference Card

```bash
# Development cycle
cargo test          # Run tests
cargo clippy        # Check lints
cargo fmt           # Format code
cargo build         # Build project
cargo run           # Run CLI

# Testing
#[tokio::test]      # Async test
.await              # Await async call
tokio_test::block_on# Doctest async wrapper

# Debugging
cargo test -- --nocapture  # Show println output
RUST_BACKTRACE=1           # Show full backtraces
cargo check                # Quick compile check
```

## Git Workflow

```bash
# Before committing
cargo fmt
cargo clippy --fix --allow-dirty
cargo test

# Commit
git add .
git commit -m "Description"

# Push
git push -u origin branch-name
```

---

**Remember**: This is an async codebase. When in doubt, make it async and await it!
