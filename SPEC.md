# Perchance Language Specification

## Overview

Perchance is a list-based random text generation language. Programs consist of named lists with items that can reference other lists. The interpreter evaluates these references by randomly selecting items, ultimately producing text output.

## Core Concepts

### Lists

A **list** is a named collection of items. Lists are defined with a name followed by indented items:

```
animal
	pig
	cow
	zebra
```

### Indentation

**Critical Rule**: Items must be indented with exactly **one tab** or **two spaces**. Indentation defines hierarchical structure.

### The Output List

The special list named `output` is the entry point for evaluation. When a Perchance program is evaluated, the `output` list is selected and evaluated.

## Syntax Elements

### 1. List References (Square Brackets)

Square brackets `[name]` reference a list and randomly select one item from it:

```
animal
	cat
	dog

output
	I saw a [animal].
```

This produces either "I saw a cat." or "I saw a dog."

### 2. Inline Lists (Curly Braces with Pipes)

Curly braces with pipe separators create inline random choices:

```
output
	That's {very|extremely} {tiny|small}!
```

**Important**: Spaces inside curly blocks are preserved. `{hi|hello}` differs from `{ hi | hello }`.

### 3. Number Ranges

Curly braces can generate random integers:

```
output
	I rolled a {1-6}.
```

This generates a random integer from 1 to 6 inclusive.

### 4. Letter Ranges

Curly braces can generate random letters:

```
output
	Random letter: {a-z}
	Random uppercase: {A-Z}
```

### 5. Probability Weights (Caret Operator)

The caret `^` adjusts selection probability:

```
animal
	common_bird^5
	rare_bird^0.1
	normal_bird
```

- `^5` makes an item 5 times more likely
- `^0.1` makes an item 0.1 times as likely (10% of normal probability)
- No weight means weight of 1.0

Weights work in inline lists too:

```
{big|large^3|massive^0.5}
```

### 6. Hierarchical Lists (Sublists)

Lists can contain sublists through indentation:

```
animal
	mammal
		dog
		cat
	bird
		crow
		sparrow
```

Access patterns:
- `[animal]` - Selects a category, then an item from that category
- `[animal.mammal]` - Directly selects from the mammal sublist
- `[animal.bird]` - Directly selects from the bird sublist

### 7. Properties

Lists can have properties using the equals sign:

```
race
	dwarf
		height = {7-15}0cm
		name = Dwarf
		description
			Short and sturdy
			Bearded and proud
	elf
		height = {15-20}0cm
		name = Elf
```

Access properties with dot notation: `[race.dwarf.height]`

### 8. Variable Assignment

Store selected values in variables using `=` inside square brackets:

```
output
	[r = race.selectOne]The [r.name] is [r.height] tall.
```

**Important**: Without `.selectOne`, assignment creates an alias to the list itself.

### 9. Commas in Square Brackets

Commas execute multiple statements, displaying only the last:

```
[a = animal.selectOne, b = color.selectOne, "I saw a [b] [a]."]
```

To execute without output:

```
[x = animal.selectOne, ""]
```

### 10. Dynamic Sub-list Referencing

Use bracket notation to reference sublists by variable values:

```
output
	[g = gender.selectOne]My name is [names[g].selectOne].

gender
	female
	male

names
	female
		Alice
		Beth
	male
		Bob
		Charlie
```

`names[g]` uses the *value* stored in `g` as the sublist name.

### 11. Comments

Double slashes `//` create comments (ignored by interpreter):

```
// This is a comment
animal  // Comments can also appear at line end
	dog
	cat
```

### 12. Escape Sequences

Special characters require escaping:

- `\s` - Preserves spaces at start/end of items
- `\t` - Tab character
- `\\` - Literal backslash
- `\[` - Literal left square bracket
- `\]` - Literal right square bracket
- `\{` - Literal left curly brace
- `\}` - Literal right curly brace
- `\=` - Literal equals sign
- `\^` - Literal caret

Example:

```
item
	\s  spaces on both sides  \s
	cost is \$100
	array\[5\] = value
```

### 13. Special Inline List Functions

#### Article Selection: `{a}`

Automatically chooses "a" or "an":

```
output
	I saw {a} [animal].
```

With "elephant" → "I saw an elephant."
With "dog" → "I saw a dog."

#### Pluralization: `{s}`

Conditionally pluralizes based on preceding number:

```
output
	You have {1-5} apple{s}.
```

"You have 1 apple." or "You have 3 apples."

## Built-in Methods

Methods are called with dot notation: `[listname.methodName]`

### Selection Methods

- **selectOne** - Explicitly selects one item from a list
- **selectAll** - Returns all items as an array
- **selectMany(n)** - Selects n items with repetition allowed
- **selectUnique(n)** - Selects n unique items without repetition

### Text Transformation Methods

- **upperCase** - Converts to uppercase
- **lowerCase** - Converts to lowercase
- **titleCase** - Capitalizes each word
- **sentenceCase** - Capitalizes first letter only

### Grammar Methods

- **pluralForm** - Converts to plural
- **singularForm** - Converts to singular
- **pastTense** - Converts verb to past tense
- **presentTense** - Converts verb to present tense
- **futureTense** - Converts verb to future tense

### List Information Methods

- **getLength** - Returns number of items in list
- **getName** - Returns the list name
- **getChildNames** - Returns names of sublists
- **getOdds** - Gets probability of selected item

### Other Methods

- **evaluateItem** - Explicitly evaluates the item
- **joinItems(separator)** - Joins array items with separator
- **replaceText(find, replace)** - Replaces text

## Evaluation Semantics

### Basic Evaluation Process

1. Start with the `output` list
2. Select one item randomly (respecting weights)
3. Evaluate the item text:
   - Replace `[list]` references with randomly selected items
   - Replace `{option1|option2}` with a random choice
   - Replace `{n-m}` with a random integer
   - Process escape sequences
4. Continue evaluating until no more references remain

### Nested Evaluation

References are evaluated recursively. If a selected item contains more references, those are evaluated in turn:

```
animal
	[color] [species]

color
	red
	blue

species
	bird
	fish

output
	I saw a [animal].
```

Produces: "I saw a red bird." (or similar combinations)

### List vs Item Selection

When a list is referenced:
- `[animal]` randomly selects an item from `animal`
- If that item has sublists, a sublist is first randomly selected, then an item from that sublist

For hierarchical lists:

```
animal
	mammal
		dog
		cat
	bird
		crow
```

`[animal]` first randomly picks `mammal` or `bird`, then randomly picks an item from that sublist.

### Property Access

Properties are accessed via dot notation and return their defined value:
- `[race.dwarf.height]` evaluates the height property
- If height = `{7-15}0cm`, this generates a random height like "120cm"

### Variable Scoping

Variables assigned in square brackets are available for the remainder of that evaluation context:

```
[x = animal.selectOne]First: [x], Second: [x]
```

Both `[x]` references will use the same animal.

## Determinism Requirement

For this implementation:
- The interpreter must accept a seeded RNG
- Given the same seed and same input, output must be identical
- All random selections (list items, inline choices, number ranges) use the provided RNG

## Out of Scope (Not Implemented)

The following features from the full Perchance system are **not** implemented in this core interpreter:

- JavaScript code execution
- Plugin system
- `$output` keyword for custom exports
- `consumableList` and list state management
- Dynamic properties and property functions
- Import/export between generators
- HTML/CSS rendering
- User input handling
- Conditional logic (if/else)
- Loops and repetition constructs
- Mathematical operations beyond number generation
- String concatenation with `+` operator

## Grammar Summary

```
program          ::= list*
list             ::= IDENTIFIER NEWLINE item+
item             ::= INDENT content NEWLINE sublist*
sublist          ::= list (further indented)
content          ::= (text | reference | inline | property)*
reference        ::= "[" expr "]"
expr             ::= IDENTIFIER accessor* | assignment ("," assignment)* ("," output)?
assignment       ::= IDENTIFIER "=" expr
output           ::= expr | STRING
accessor         ::= "." IDENTIFIER | "[" expr "]"
inline           ::= "{" choice ("|" choice)* "}"
choice           ::= content weight?
weight           ::= "^" NUMBER
property         ::= IDENTIFIER "=" content
text             ::= (CHAR | escape)+
escape           ::= "\\" CHAR
comment          ::= "//" .* NEWLINE

INDENT           ::= TAB | "  " (two spaces)
IDENTIFIER       ::= [a-zA-Z_][a-zA-Z0-9_]*
NUMBER           ::= [0-9]+ ("." [0-9]+)?
STRING           ::= '"' .* '"'
```

## Example Programs

### Simple Example

```
animal
	pig
	cow
	zebra

output
	I saw a [animal] today!
```

### Weighted Example

```
rarity
	common^10
	uncommon^3
	rare^1
	legendary^0.1

output
	You found a [rarity] item!
```

### Hierarchical Example

```
creature
	land
		mammal
			dog
			cat
		reptile
			lizard
			snake
	water
		fish
			salmon
			tuna
		mammal
			whale
			dolphin

output
	The [creature] is magnificent!
```

### Properties Example

```
character
	wizard
		name = Gandalf
		power = {80-100}
		type = Magic User
	warrior
		name = Conan
		power = {60-90}
		type = Fighter

output
	[c = character.selectOne]Name: [c.name]
	Type: [c.type]
	Power Level: [c.power]
```

### Dynamic Reference Example

```
gender
	male
	female

names
	male
		Bob
		Jim
	female
		Alice
		Jane

output
	[g = gender.selectOne]Hello, I'm [names[g].selectOne] and I'm [g].
```

## Implementation Notes

### Parser Requirements

- Tokenize input handling indentation, comments, and escape sequences
- Build Abstract Syntax Tree (AST) representing lists, items, and expressions
- Validate indentation consistency
- Track line numbers for error reporting

### Compiler Requirements

- Transform AST into an evaluatable representation
- Resolve list references and validate they exist
- Prepare weight-based selection structures
- Optimize for repeated evaluation

### Evaluator Requirements

- Accept compiled program and RNG instance
- Evaluate `output` list recursively
- Handle all reference types (direct, property, dynamic)
- Apply methods correctly
- Maintain variable scope during evaluation

### RNG Integration

- Use Rust's `rand` crate traits
- Accept any type implementing `rand::Rng`
- Use weighted selection for items with `^` weights
- Use range generation for `{n-m}` syntax

### Error Handling

- Parse errors: Invalid syntax, bad indentation
- Compile errors: Undefined list references, invalid property access
- Runtime errors: Division by zero (if arithmetic added), type mismatches
- Provide clear error messages with line numbers
