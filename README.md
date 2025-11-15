# Perchance Interpreter

[![CI](https://github.com/philpax/perchance-interpreter/workflows/CI/badge.svg)](https://github.com/philpax/perchance-interpreter/actions/workflows/ci.yml)
[![Frontend CI](https://github.com/philpax/perchance-interpreter/workflows/Frontend%20CI/badge.svg)](https://github.com/philpax/perchance-interpreter/actions/workflows/frontend.yml)
[![Deploy Frontend](https://github.com/philpax/perchance-interpreter/workflows/Deploy%20Frontend/badge.svg)](https://github.com/philpax/perchance-interpreter/actions/workflows/deploy-frontend.yml)

A Rust implementation of the [Perchance](https://perchance.org/) template language with deterministic random generation. Includes a web frontend powered by WebAssembly.

## What is Perchance?

Perchance is a random text generator language that uses weighted lists and template expansion to create procedurally generated content. This interpreter implements the core Perchance functionality in Rust with a focus on deterministic output, making it suitable for embedding in games, tools, and applications.

## What's Implemented

This is a **fully-functional** implementation of core Perchance features:

### Core Features ✅
- **List selection** with weighted randomization (`item^3`)
- **Inline lists** (`{option1|option2}`)
- **Ranges** (`{1-10}`, `{a-z}`, including negatives)
- **Hierarchical lists** with property access (`[character.wizard.name]`)
- **Variables** and sequences (`[x = animal, x]`)
- **Methods**: `upperCase`, `lowerCase`, `titleCase`, `sentenceCase`, `selectOne`, `selectMany(n)`, `selectUnique(n)`, `selectAll`, `joinItems(sep)`
- **Grammar methods**: `pluralForm`, `pastTense`, `possessiveForm`, `futureTense`, `presentTense`, `negativeForm`, `singularForm`
- **Special functions**: `{a}` (smart articles), `{s}` (pluralization)
- **Conditional logic**: ternary operator (`[x > 5 ? "big" : "small"]`) with binary operators (`==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`)
- **consumableList**: stateful lists where items are removed after selection
- **$output keyword**: custom list output without random selection
- **Import/export**: `{import:generator-name}` with property access
- **37 builtin generators**: animals, colors, nouns, countries, and more

### Advanced Features ✅
- Deterministic RNG with seeded generation
- Dynamic sub-list referencing (`[list[variable]]`)
- Chained property access
- Escape sequences (`\s`, `\t`, `\\`, `\[`, `\]`, `\{`, `\}`, `\=`, `\^`)
- Comments with `//`
- Two-space or tab indentation

### Test Status: **180/180 tests passing** ✨

## What Remains to be Done

Features not yet implemented:

- **JavaScript execution** - Inline JavaScript code blocks
- **Plugin system** - Custom plugin loading
- **`this` keyword** - Full support with property assignment syntax
- **HTML/CSS rendering** - Proper handling of HTML tags and styles
- **Math operations** - Expression evaluation (`+`, `-`, `*`, `/`, `%`)
- **String concatenation** - `+` operator for strings
- **Additional features**:
  - Dynamic odds with `^[condition]` syntax
  - `evaluateItem` method for explicit evaluation before storage
  - Property fallback `||` operator (e.g., `[a.property || "default"]`)
  - Variable-count selection (`selectMany(min, max)`, `selectUnique(min, max)`)

**Note**: Binary `||` IS supported in conditionals (e.g., `[a || b ? "yes" : "no"]`)

## Installation

```bash
cargo build --release
```

## Usage

### As a Library

```rust
use perchance_interpreter::run_with_seed;

let template = r#"
animal
	dog
	cat
	bird

output
	I saw a [animal].
"#;

let result = run_with_seed(template, 42, None).await.unwrap();
println!("{}", result);
```

### CLI Tool

```bash
# Evaluate with a seed (deterministic)
cargo run --bin perchance template.perchance 42

# Random output
cargo run --bin perchance template.perchance

# Read from stdin
cat template.perchance | cargo run --bin perchance -
```

### Web Frontend

A modern, interactive web interface is available at [https://philpax.github.io/static/experimental/perchance/](https://philpax.github.io/static/experimental/perchance/)

**Run locally:**
```bash
# Quick start (builds WASM + starts dev server)
python build-frontend.py --dev

# Or manually
wasm-pack build perchance-wasm --target web --out-dir ../frontend/src/wasm
cd frontend && npm install && npm run dev
```

Features:
- Live preview as you type
- Generate multiple samples
- Syntax highlighting
- Clear error messages
- Responsive design with Tailwind CSS

See [frontend/README.md](frontend/README.md) for details.

## Quick Examples

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
	rare

output
	Found a [rarity] item!
```

### Variables & Methods
```
name
	alice
	bob

output
	[n = name, n.titleCase] and [n.upperCase]
```

### Conditionals
```
number
	{1-10}

output
	[n = number, n > 5 ? "High" : "Low"]
```

### Imports
```
output
	I saw {a} {import:animal}.
```

### consumableList (No Duplicates)
```
card
	ace
	king
	queen
	jack

output
	[deck = card.consumableList][deck], [deck], [deck]
```
Output: `"king, ace, jack"` (each card appears once)

## Testing

```bash
cargo test              # Run all tests
cargo clippy            # Lint
cargo fmt               # Format
```

## Architecture

```
Template → Parser → AST → Compiler → CompiledProgram → Evaluator → Output
```

- **Parser** (`src/parser.rs`): Text → AST
- **Compiler** (`src/compiler.rs`): AST → Optimized program with pre-calculated weights
- **Evaluator** (`src/evaluator.rs`): Async execution with RNG and import support
- **Loader** (`src/loader.rs`): Import system with builtin generators
- **CLI** (`src/bin/main.rs`): Command-line interface
- **WASM** (`perchance-wasm/`): WebAssembly bindings for frontend

## Builtin Generators

37 generators included: `animal`, `color`, `noun`, `country`, `occupation`, `fruit`, `vegetable`, `emotion`, `greek-god`, `planet-name`, and more. See `src/builtin_generators/mod.rs` for the full list.

## Future Work

Additional features and enhancements:

1. **Long-form if/else** - `[if (cond) {a} else {b}]` syntax alongside ternary
2. **String manipulation** - Methods like `substring`, `replace`, `trim`, `split`
3. **Advanced grammar** - Comparative/superlative forms, better edge cases for irregular words
4. **Number formatting** - Number-to-word conversion, ordinals (1st, 2nd, 3rd)
5. **Performance optimizations** - Further compiler optimizations, caching strategies

## Contributing

Contributions welcome! Please:
1. Run `cargo test && cargo clippy && cargo fmt` before committing
2. Add tests for new features
3. Update documentation

## License

MIT
