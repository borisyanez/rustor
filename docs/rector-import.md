# Importing Rules from Rector

Rustor includes a tool to import refactoring rules from [Rector](https://github.com/rectorphp/rector), a popular PHP refactoring tool. This allows rustor to leverage Rector's extensive collection of battle-tested transformation rules.

## Overview

The `rustor-import-rector` tool:

1. Parses Rector PHP rule files
2. Extracts rule metadata (name, description, node types, code samples)
3. Detects common refactoring patterns
4. Generates equivalent Rust implementations

## Installation

```bash
# Build from source
cd rustor
cargo build -p rustor-rector-import --release

# The binary is at target/release/rustor-import-rector
```

---

## Quick Start

### 1. Get Rector Source Code

```bash
git clone --depth 1 https://github.com/rectorphp/rector-src.git /tmp/rector
```

### 2. Generate Compatibility Report

```bash
rustor-import-rector report -r /tmp/rector
```

This shows:
- Total rules found
- How many can be auto-generated
- Pattern distribution
- Category breakdown

### 3. Generate Rules

```bash
# Generate only auto-generatable rules
rustor-import-rector generate -r /tmp/rector -o ./imported/ --auto-only

# Preview without writing files
rustor-import-rector generate -r /tmp/rector -o ./imported/ --auto-only --dry-run
```

---

## Commands

### `rustor-import-rector report`

Generate a compatibility report showing which Rector rules can be imported.

```bash
rustor-import-rector report -r <RECTOR_PATH> [OPTIONS]

Options:
  -r, --rector-path <PATH>   Path to Rector repository (required)
  -f, --format <FORMAT>      Output format: terminal, markdown, json (default: terminal)
  -o, --output <FILE>        Write report to file (default: stdout)
```

**Example output:**

```
════════════════════════════════════════════════════════════
  Rector Rule Import Report
════════════════════════════════════════════════════════════

Summary
  Total rules found:     468
  Auto-generatable:      20
  Needs manual review:   448
  Failed to parse:       0

  Auto-generation coverage: [████░░░░░░░░░░░░░░░░░░░░░░░░░░] 4.3%

Pattern Distribution
  ✓ FunctionRename            9
  ✓ FunctionToComparison      3
  ✓ ArraySyntaxModern         3
  ✓ ClosureToArrow            2
  ✓ FunctionAlias             2
  ✓ FunctionToOperator        1
  ○ Complex                   157
  ✗ Unknown                   291
```

### `rustor-import-rector list`

List all rules that can be automatically generated.

```bash
rustor-import-rector list -r <RECTOR_PATH> [OPTIONS]

Options:
  -r, --rector-path <PATH>   Path to Rector repository (required)
  -c, --category <NAME>      Filter by category (e.g., Php80, CodeQuality)
```

**Example output:**

```
Found 20 auto-generatable rules:

  ✓ PowToExpRector [FunctionToOperator] - Changes `pow(val, val2)` to `**`...
  ✓ SizeofToCountRector [FunctionAlias] - Changes sizeof() to count()
  ✓ JoinToImplodeRector [FunctionAlias] - Changes join() to implode()
  ✓ IsNullToIdenticalRector [FunctionToComparison] - Changes is_null($x)...
```

### `rustor-import-rector generate`

Generate Rust rule files from Rector rules.

```bash
rustor-import-rector generate -r <RECTOR_PATH> -o <OUTPUT_DIR> [OPTIONS]

Options:
  -r, --rector-path <PATH>   Path to Rector repository (required)
  -o, --output <DIR>         Output directory (required)
  -c, --category <NAME>      Only generate rules from this category
      --auto-only            Only generate auto-generatable rules
      --dry-run              Show what would be generated without writing
```

**Generated files:**

```
output/
├── mod.rs                          # Module exports
├── pow_to_exp.rs                   # FunctionToOperator pattern
├── sizeof_to_count.rs              # FunctionAlias pattern
├── is_null_to_identical.rs         # FunctionToComparison pattern
└── long_array_to_short_array.rs    # ArraySyntaxModern pattern
```

### `rustor-import-rector analyze`

Analyze a single Rector rule file in detail.

```bash
rustor-import-rector analyze <FILE>
```

**Example output:**

```
→ Analyzing: rules/Php56/Rector/FuncCall/PowToExpRector.php

Rule Information
  Name:          PowToExpRector
  Category:      Php56
  Description:   Changes `pow(val, val2)` to `**` (exp) parameter
  Node Types:    FuncCall
  PHP Version:   5.6
  Configurable:  false

Pattern Analysis
  Type:          FunctionToOperator
  Auto-Gen:      Yes

Code Sample
  Before: pow(1, 2);
  After:  1**2;
```

---

## Supported Patterns

The importer can automatically generate Rust code for these patterns:

### FunctionRename

Renames a function call to a different function.

```php
// Before
join(',', $array);

// After
implode(',', $array);
```

**Detection:** Matches `$this->isName($node, 'old_name')` combined with `new Name('new_name')`.

### FunctionAlias

Special case of rename for known PHP function aliases.

| Alias | Canonical |
|-------|-----------|
| `sizeof` | `count` |
| `join` | `implode` |
| `chop` | `rtrim` |
| `pos` | `current` |
| `key_exists` | `array_key_exists` |
| `is_integer` | `is_int` |
| `is_double` | `is_float` |

### FunctionToComparison

Converts a function call to a comparison expression.

```php
// Before
is_null($value);

// After
$value === null;
```

### FunctionToCast

Converts a function call to a type cast.

```php
// Before
strval($value);

// After
(string) $value;
```

**Supported conversions:**
- `strval()` → `(string)`
- `intval()` → `(int)`
- `floatval()` → `(float)`
- `boolval()` → `(bool)`

### FunctionToOperator

Converts a function call to an operator expression.

```php
// Before
pow($base, $exp);

// After
$base ** $exp;
```

### TernaryToCoalesce

Converts isset/empty ternary to null coalescing operator.

```php
// Before
isset($value) ? $value : 'default';

// After
$value ?? 'default';
```

### ArraySyntaxModern

Converts legacy array syntax to short syntax.

```php
// Before
array(1, 2, 3);

// After
[1, 2, 3];
```

### ClosureToArrow

Converts simple closures to arrow functions (PHP 7.4+).

```php
// Before
array_map(function($x) { return $x * 2; }, $arr);

// After
array_map(fn($x) => $x * 2, $arr);
```

---

## Generated Code Structure

Each generated rule follows this structure:

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
//!
//! Imported from Rector: {source_file}

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};
use crate::registry::{Category, PhpVersion, Rule};

pub fn check_{snake_name}<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = {PascalName}Visitor { source, edits: Vec::new() };
    visitor.visit_program(program, source);
    visitor.edits
}

struct {PascalName}Visitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for {PascalName}Visitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Pattern-specific matching logic
        true
    }
}

pub struct {PascalName}Rule;

impl Rule for {PascalName}Rule {
    fn name(&self) -> &'static str { "{snake_name}" }
    fn description(&self) -> &'static str { "{description}" }
    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_{snake_name}(program, source)
    }
    fn category(&self) -> Category { Category::{Category} }
    fn min_php_version(&self) -> Option<PhpVersion> { {php_version} }
}

#[cfg(test)]
mod tests {
    // Auto-generated from Rector code samples
}
```

---

## Integrating Generated Rules

After generating rules, integrate them into rustor:

### 1. Copy to rustor-rules

```bash
cp -r ./imported/* crates/rustor-rules/src/imported/
```

### 2. Add module declaration

In `crates/rustor-rules/src/lib.rs`:

```rust
pub mod imported;
```

### 3. Register rules

In `crates/rustor-rules/src/registry.rs`:

```rust
use crate::imported;

// In RuleRegistry::new_with_config()
for rule in imported::imported_rules() {
    registry.register(rule);
}
```

### 4. Test

```bash
cargo test -p rustor-rules imported
```

---

## Complex Rules

Rules marked as "Complex" or "Unknown" cannot be auto-generated because they:

- Require PHPStan type information
- Perform multi-statement analysis
- Need class/method context
- Have custom configuration
- Use complex control flow

For these rules, the generator creates a skeleton with:

```rust
fn visit_expression(&mut self, _expr: &Expression<'a>, _source: &str) -> bool {
    // TODO: Manual implementation required
    // Hints from Rector analysis:
    // - Uses type checking
    // - Traverses child nodes
    //
    // Please refer to the original Rector rule for implementation details.
    true
}
```

---

## AST Node Mapping

The importer maps PHP-Parser node types to mago-syntax equivalents:

| PHP-Parser | mago-syntax |
|------------|-------------|
| `Expr\FuncCall` | `Expression::Call(Call::Function(_))` |
| `Expr\MethodCall` | `Expression::Call(Call::Method(_))` |
| `Expr\StaticCall` | `Expression::Call(Call::StaticMethod(_))` |
| `Expr\BinaryOp\Identical` | `Expression::Binary(_)` with `BinaryOperator::Identical` |
| `Expr\Variable` | `Expression::Variable(Variable::Direct(_))` |
| `Expr\Array_` | `Expression::Array(_)` |
| `Expr\Ternary` | `Expression::Conditional(_)` |
| `Expr\Closure` | `Expression::Closure(_)` |
| `Stmt\Class_` | `Statement::Class(_)` |
| `Stmt\ClassMethod` | `ClassLikeMember::Method(_)` |

See `ast_mapper.rs` for the complete mapping table.

---

## Troubleshooting

### "Rules directory not found"

Make sure you're pointing to the correct Rector repository:

```bash
# Should contain: rules/CodeQuality/, rules/Php80/, etc.
ls /path/to/rector/rules/
```

### "No auto-generatable rules found"

This means all rules in the specified category use complex patterns. Try:

```bash
# List all categories
ls /path/to/rector/rules/

# Try a different category
rustor-import-rector list -r /path/to/rector -c Php56
```

### Generated code doesn't compile

Some generated code may need minor adjustments:

1. Check import statements
2. Verify span extraction logic
3. Ensure argument handling matches mago-syntax API

---

## See Also

- [Development Guide](development.md) - How to add rules manually
- [Rules Reference](rules.md) - Existing rustor rules
- [Rector Documentation](https://getrector.org/documentation) - Rector's rule documentation
