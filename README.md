# Perchance Interpreter

A Rust implementation of the Perchance template language with deterministic random generation.

## Features

This interpreter implements core Perchance functionality:

### ✅ Implemented
- Basic list selection
- Weighted selection with `^` operator
- Inline lists with `{option1|option2}` syntax
- Number ranges `{1-10}`, including negative ranges `{-5-5}`
- Letter ranges `{a-z}`, `{A-Z}`
- Escape sequences (`\s`, `\t`, `\\`, `\[`, `\]`, `\{`, `\}`, `\=`, `\^`)
- Comments with `//`
- Deterministic random generation with seeded RNG
- Two-space or tab indentation
- Multiple list references
- Nested inline lists
- Variable assignment and references in sequences `[x = animal, x]`
- String literals in expressions (evaluated for references)
- Text transformation methods (`upperCase`, `lowerCase`, `titleCase`, `sentenceCase`)
- **Hierarchical lists** - Full support for nested sublists
- **Property access** - Chained property access like `[character.wizard.name]`
- **Methods without parentheses** - `[word.upperCase]` works correctly

### 🚧 Partially Implemented
- `selectOne` method with property access - Returns string instead of value (1 failing test)
- Dynamic sub-list referencing with `[list[variable]]`

### ❌ Not Implemented (Out of Scope)
- JavaScript code execution
- Plugin system
- `$output` keyword
- `consumableList` and stateful lists
- HTML/CSS rendering
- Import/export between generators
- Conditional logic (if/else)
- Loops and repetition
- Grammar methods (`pluralForm`, `pastTense`, etc.)
- Special inline functions (`{a}`, `{s}`)

## Installation

```bash
cargo build --release
```

## Usage

### As a Library

```rust
use perchance_interpreter::evaluate_with_seed;

let template = "animal\n\tdog\n\tcat\n\noutput\n\tI saw a [animal].\n";
let result = evaluate_with_seed(template, 42).unwrap();
println!("{}", result);
```

### CLI Tool

```bash
# Evaluate a template file with a seed
cargo run --bin perchance template.perchance 42

# Random output (no seed)
cargo run --bin perchance template.perchance

# Read from stdin
cat template.perchance | cargo run --bin perchance -
```

## Template Syntax

### Basic Lists

```
animal
	dog
	cat
	bird

output
	I saw a [animal].
```

### Weighted Selection

```
rarity
	common^10
	uncommon^3
	rare^1

output
	You found a [rarity] item!
```

### Inline Lists

```
output
	That's a {very|extremely} {big|small} animal!
```

### Number and Letter Ranges

```
output
	I rolled a {1-6}.
	Random letter: {a-z}
```

### Variables

```
animal
	dog
	cat

output
	[x = animal, x] and [x]
```

## Testing

```bash
# Run all tests
cargo test

# Run only integration tests
cargo test --test integration_tests
```

## Known Issues

1. **selectOne with property access**: `[c = character.selectOne, c.name]` doesn't preserve properties. The selectOne method returns an evaluated string instead of keeping the value structure for further property access.

## Architecture

- **Parser** (`src/parser.rs`): Converts text to AST
- **Compiler** (`src/compiler.rs`): Transforms AST into evaluatable structure
- **Evaluator** (`src/evaluator.rs`): Executes with RNG to produce output
- **CLI** (`src/bin/main.rs`): Command-line interface

## Test Results

Current test status: **36 out of 37 integration tests passing (97%)**

All categories working:
- ✅ Basic list selection and determinism
- ✅ Weighted selection
- ✅ Inline lists with weights
- ✅ Number/letter ranges (including negative)
- ✅ Escape sequences
- ✅ Comments
- ✅ Variable assignment in sequences
- ✅ String literals with references
- ✅ **Hierarchical lists** (all depths)
- ✅ **Property access** (chained)
- ✅ **Methods** (upperCase, lowerCase, titleCase, sentenceCase)
- ⚠️ selectOne method with subsequent property access (1 test)

## Future Work

To complete the core implementation:
1. Fix selectOne to return Value instead of String for property chaining
2. Add dynamic sub-list referencing support `[list[variable]]`
3. Implement remaining methods (pluralForm, pastTense, futureTense, etc.)
4. Add special inline functions (`{a}`, `{s}` for grammar)
5. Implement `selectMany`, `selectUnique`, `selectAll` methods

## License

MIT
