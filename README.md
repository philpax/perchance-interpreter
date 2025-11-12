# Perchance Interpreter

[![CI](https://github.com/philpax/perchance-interpreter/workflows/CI/badge.svg)](https://github.com/philpax/perchance-interpreter/actions/workflows/ci.yml)
[![Frontend CI](https://github.com/philpax/perchance-interpreter/workflows/Frontend%20CI/badge.svg)](https://github.com/philpax/perchance-interpreter/actions/workflows/frontend.yml)

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
- **selectOne method** - Full support including property access `[c = list.selectOne, c.property]`
- **Dynamic sub-list referencing** - `[list[variable]]` for computed property access
- **Selection methods** - `selectMany(n)`, `selectUnique(n)`, `selectAll` for bulk selection
- **Special inline functions** - `{a}` for smart articles, `{s}` for pluralization
- **Grammar methods** - `pluralForm`, `pastTense`, `possessiveForm`, `futureTense`, `presentTense`, `negativeForm`, `singularForm`
- **joinItems method** - Custom separators for list results: `[list.selectMany(3).joinItems(", ")]`
- **Conditional logic** - Ternary operator `[condition ? true : false]`
- **Binary operators** - `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`
- **$output keyword** - Custom output for lists without random selection
- **consumableList** - Stateful lists where items are removed after selection

### ❌ Not Implemented (Out of Scope)
- JavaScript code execution
- Plugin system
- `this` keyword for accessing sibling properties
- HTML/CSS rendering (HTML tags are passed through as-is)
- Import/export between generators with `{import:name}` syntax (requires multi-file architecture)
- Long-form if/else statements (ternary `?:` is supported)
- Mathematical operations (+, -, *, /, etc.)
- String concatenation with `+` operator
- **Dynamic odds** with `^[condition]` syntax (e.g., `item ^[variable == "value"]`)
- **`evaluateItem` method** for explicitly evaluating items before storage
- **`||` operator for property fallback** (e.g., `[a.property || "default"]` for missing properties)
- **Variable-count selection** - `selectMany(min, max)` and `selectUnique(min, max)` for random counts

**Note**: Binary `||` operator IS supported in conditional expressions (e.g., `[a || b ? "yes" : "no"]`)

## Installation

```bash
cargo build --release
```

## Web Frontend

This project includes a beautiful web-based frontend built with React, TypeScript, and WebAssembly. The frontend provides:

- **Live Preview**: Real-time evaluation as you type
- **Interactive Editor**: Syntax-highlighted editor with auto-completion
- **Multiple Samples**: Generate many outputs from the same template
- **Error Display**: Clear, helpful error messages
- **Modern UI**: Responsive design with Tailwind CSS

### Running the Frontend

#### Quick Start (Recommended)

Use the cross-platform Python build script:

```bash
# Setup and start development server
python build-frontend.py --dev

# Or just build for production
python build-frontend.py --build
```

#### Manual Setup

1. Build the WASM module:
   ```bash
   wasm-pack build perchance-wasm --target web --out-dir ../frontend/src/wasm
   ```

2. Start the development server:
   ```bash
   cd frontend
   npm install
   npm run dev
   ```

3. Open http://localhost:5173 in your browser

For more details, see [frontend/README.md](frontend/README.md).

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

### Special Inline Functions

```
output
	I saw {a} elephant and {a} dog.
	I have 1 apple{s} and 3 orange{s}.
```

Output: "I saw an elephant and a dog. I have 1 apple and 3 oranges."

### Grammar Methods

```
noun
	child
	city

verb
	run
	walk

name
	James

output
	The [noun.pluralForm] [verb.pastTense].
	[name.possessiveForm] book.
```

Output: "The children ran. James' book."

### Selection Methods

```
color
	red
	blue
	green

output
	[color.selectMany(3)]
	[color.selectUnique(2)]
	[color.selectAll]
```

### Conditional Logic

```
number
	{1-6}

output
	[n = number, n < 4 ? "Too bad" : "Nice!"]
	[n > 2 && n < 5 ? "Middle" : "Edge"]
	[n == 6 ? "Perfect!" : "Keep trying"]
```

### $output Keyword

```
greeting
	hello
	hi
	hey
	$output = Welcome to our service

output
	[greeting]
```

Output: "Welcome to our service" (always the same, no random selection)

### joinItems with Custom Separator

```
fruit
	apple
	banana
	orange

output
	[fruit.selectMany(3).joinItems(", ")]
```

Output: "banana, banana, orange" (or similar with comma separation)

### consumableList for Unique Selection

```
card
	ace
	king
	queen
	jack

output
	[deck = card.consumableList][deck], [deck], [deck], [deck]
```

Output: "king, ace, jack, queen" (each card appears exactly once, in random order)

## Testing

```bash
# Run all tests
cargo test

# Run only integration tests
cargo test --test integration_tests
```

## Known Issues

None! All core features are working correctly.

## Architecture

- **Parser** (`src/parser.rs`): Converts text to AST
- **Compiler** (`src/compiler.rs`): Transforms AST into evaluatable structure
- **Evaluator** (`src/evaluator.rs`): Executes with RNG to produce output
- **CLI** (`src/bin/main.rs`): Command-line interface

## Test Results

Current test status: **106 out of 109 integration tests passing (97%)** ✨

**3 failing tests** (expected - features not yet implemented):
- `test_multiline_dynamic_odds_with_equality` - Dynamic odds `^[condition]` syntax
- `test_multiline_evaluate_item_with_ranges` - `evaluateItem` method
- `test_multiline_or_operator_fallback` - Property fallback `||` operator

All implemented categories working:
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
- ✅ **Text transformation methods** (upperCase, lowerCase, titleCase, sentenceCase)
- ✅ **Selection methods** (selectOne, selectMany, selectUnique, selectAll)
- ✅ **Special inline functions** ({a} for articles, {s} for pluralization)
- ✅ **Grammar methods** (pluralForm, pastTense, possessiveForm, futureTense, presentTense, negativeForm, singularForm)
- ✅ **joinItems method** (custom separators for lists)
- ✅ **Conditional logic** (ternary operator, binary operators)
- ✅ **$output keyword** (custom list output)
- ✅ **consumableList** (stateful unique selection)

## Future Work

Potential enhancements:
1. **Import/export system** - `{import:name}` syntax for composing generators (requires multi-file architecture)
2. **`this` keyword** - Complete implementation with property assignment syntax (e.g., `property = value`)
3. **Long-form if/else** - `[if (cond) {a} else {b}]` syntax alongside ternary
4. Add more sophisticated article selection (handle words like "university", "hour")
5. Add number-to-word conversion method
6. Implement string manipulation methods (substring, replace, trim, etc.)
7. Add mathematical expression evaluation (+, -, *, /, etc.)
8. Improve plural/past tense rules for edge cases
9. Add comparative/superlative grammar forms

## License

MIT
