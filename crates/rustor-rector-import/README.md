# rustor-rector-import

Import [Rector](https://github.com/rectorphp/rector) PHP refactoring rules into rustor.

This crate analyzes Rector PHP rule files and automatically generates equivalent rustor Rust rules. It enables rustor to leverage Rector's battle-tested refactoring patterns.

## Features

- **PHP File Parser** - Regex-based extraction of rule metadata from Rector PHP files
- **Pattern Detection** - Recognizes 8 common rule patterns for automatic code generation
- **Code Generation** - Generates complete Rust rule implementations with tests
- **Compatibility Reports** - Terminal, Markdown, and JSON reports
- **CLI Tool** - Full-featured command-line interface

## Installation

```bash
cargo build -p rustor-rector-import --release
```

The binary `rustor-import-rector` will be available in `target/release/`.

## Quick Start

```bash
# Clone Rector repository
git clone --depth 1 https://github.com/rectorphp/rector-src.git /tmp/rector

# Generate compatibility report
rustor-import-rector report -r /tmp/rector

# List auto-generatable rules
rustor-import-rector list -r /tmp/rector

# Generate Rust rules
rustor-import-rector generate -r /tmp/rector -o ./imported-rules/ --auto-only

# Analyze a single rule
rustor-import-rector analyze /tmp/rector/rules/Php56/Rector/FuncCall/PowToExpRector.php
```

## Commands

### `report` - Generate Compatibility Report

```bash
# Terminal output (default)
rustor-import-rector report -r /path/to/rector

# Markdown format
rustor-import-rector report -r /path/to/rector --format markdown -o report.md

# JSON format
rustor-import-rector report -r /path/to/rector --format json -o report.json
```

### `list` - List Compatible Rules

```bash
# List all auto-generatable rules
rustor-import-rector list -r /path/to/rector

# Filter by category
rustor-import-rector list -r /path/to/rector -c Php80
```

### `generate` - Generate Rust Rules

```bash
# Generate only auto-generatable rules
rustor-import-rector generate -r /path/to/rector -o ./output/ --auto-only

# Generate all rules (including skeletons for complex ones)
rustor-import-rector generate -r /path/to/rector -o ./output/

# Dry run - see what would be generated
rustor-import-rector generate -r /path/to/rector -o ./output/ --dry-run

# Filter by category
rustor-import-rector generate -r /path/to/rector -o ./output/ -c CodeQuality
```

### `analyze` - Analyze Single Rule

```bash
rustor-import-rector analyze /path/to/SomeRector.php
```

Output includes:
- Rule name and category
- Description and node types
- Detected pattern type
- Auto-generation status
- Code samples

## Recognized Patterns

The following patterns can be automatically converted to Rust rules:

| Pattern | Description | Example |
|---------|-------------|---------|
| `FunctionRename` | Rename a function | `join()` → `implode()` |
| `FunctionAlias` | Known PHP function aliases | `sizeof()` → `count()` |
| `FunctionToComparison` | Function to comparison | `is_null($x)` → `$x === null` |
| `FunctionToCast` | Function to type cast | `strval($x)` → `(string) $x` |
| `FunctionToOperator` | Function to operator | `pow($x, 2)` → `$x ** 2` |
| `TernaryToCoalesce` | Ternary to null coalesce | `isset($x) ? $x : $d` → `$x ?? $d` |
| `ArraySyntaxModern` | Legacy array to short syntax | `array()` → `[]` |
| `ClosureToArrow` | Closure to arrow function | `function($x) { return $x; }` → `fn($x) => $x` |

## Generated Code Structure

For each rule, the generator creates:

```rust
//! Rule: {description}
//!
//! Example:
//! ```php
//! // Before
//! {before_code}
//!
//! // After
//! {after_code}
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

pub fn check_{snake_name}<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    // Visitor implementation
}

pub struct {PascalName}Rule;

impl Rule for {PascalName}Rule {
    fn name(&self) -> &'static str { "{snake_name}" }
    fn description(&self) -> &'static str { "{description}" }
    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_{snake_name}(program, source)
    }
}

#[cfg(test)]
mod tests {
    // Auto-generated test from Rector code sample
}
```

## Output Files

```
output/
├── mod.rs                    # Module declarations and imported_rules() function
├── pow_to_exp.rs             # Individual rule file
├── join_to_implode.rs
├── sizeof_to_count.rs
└── ...
```

## Architecture

```
rustor-rector-import/
├── Cargo.toml
└── src/
    ├── lib.rs              # Core types (RectorRule, RulePattern, ImportResult)
    ├── main.rs             # CLI entry point
    ├── php_parser.rs       # Regex-based PHP file parser
    ├── rule_extractor.rs   # Extracts rules from PHP files
    ├── pattern_detector.rs # Detects rule patterns from refactor() body
    ├── ast_mapper.rs       # Maps PHP-Parser AST types to mago-syntax
    ├── codegen.rs          # Generates Rust code from templates
    ├── templates.rs        # Handlebars templates for code generation
    └── report.rs           # Report generation (terminal, markdown, JSON)
```

## Current Statistics

Tested against Rector repository (rector-src):

| Metric | Count |
|--------|-------|
| Total rules analyzed | 468 |
| Auto-generatable | 20 (4.3%) |
| Complex patterns | 157 |
| Unknown patterns | 291 |
| Parse failures | 0 |

## Limitations

- **Regex-based parsing** - Uses pattern matching instead of a full PHP parser
- **Simple patterns only** - Complex rules with type inference or multi-file analysis require manual implementation
- **No PHPStan integration** - Rules depending on type information cannot be auto-generated
- **Configurable rules** - Rules with configuration options need manual adaptation

## Contributing

To improve pattern detection:

1. Analyze a new pattern in `pattern_detector.rs`
2. Add a template in `templates.rs`
3. Handle the pattern in `codegen.rs`
4. Add tests for the new pattern

See the main [Development Guide](../../docs/development.md) for contribution guidelines.

## License

MIT License - see [LICENSE](../../LICENSE) for details.
