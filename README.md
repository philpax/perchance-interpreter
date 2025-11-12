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
- `this` keyword for accessing sibling properties (partially implemented, needs property assignment syntax)
- HTML/CSS rendering
- Import/export between generators with `{import:name}` syntax (requires multi-file architecture)
- Long-form if/else statements (ternary is supported)

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

Current test status: **89 out of 89 integration tests passing (100%)** ✨

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
