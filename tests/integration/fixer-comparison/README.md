# Fixer Comparison Test

Integration test comparing rustor's fixer output with PHP-CS-Fixer.

## Prerequisites

- PHP-CS-Fixer: `composer global require friendsofphp/php-cs-fixer`
- Rustor: `cargo build --release`

## Usage

```bash
# Run comparison
./run_comparison.sh

# Run with verbose output (show diffs)
./run_comparison.sh --verbose
```

## Test Files

| File | Tests |
|------|-------|
| 01_whitespace.php | Trailing whitespace, blank lines, indentation |
| 02_braces.php | Brace positioning, control structure spacing |
| 03_operators.php | Binary operators, ternary, concatenation |
| 04_imports.php | Use statements, ordering, grouping |
| 05_casing.php | Keywords, constants, magic methods |
| 06_functions.php | Function declarations, arguments, return types |
| 07_arrays.php | Array syntax, spacing, trailing commas |
| 08_declare.php | Declare statements, strict types |
| 09_classes.php | Class structure, properties, traits |
| 10_comments.php | Comment styles, PHPDoc |
| 11_control_structures.php | If/else, loops, switch, try/catch |
| 12_strings.php | Single/double quotes, concatenation |
| 13_visibility.php | Property/method visibility modifiers |
| 14_mixed.php | Combined scenarios |

## Configuration

The test uses `.php-cs-fixer.php` with:
- `@PSR12` preset
- `braces_position`: all on same line
- `single_quote`: enabled

## Current Status

**5 files identical, 9 files with differences**

Identical:
- 01_whitespace.php
- 02_braces.php
- 05_casing.php
- 08_declare.php
- 10_comments.php

Known differences (edge cases):
- Short ternary `?:` spacing
- Import ordering after group splitting
- Union type function braces
- Property indentation when splitting declarations
- Alternative syntax conversion edge cases
