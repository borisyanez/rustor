# rustor-rules

PHP refactoring rules for rustor.

## Overview

This crate contains all 23 refactoring rules organized into four categories:

| Category | Description | Rules |
|----------|-------------|-------|
| Performance | Runtime performance improvements | 5 |
| Modernization | Syntax modernization for newer PHP | 13 |
| Simplification | Code simplification | 3 |
| Compatibility | Compatibility and best practices | 2 |

## Usage

```rust
use rustor_rules::{RuleRegistry, Preset, PhpVersion};
use std::collections::HashSet;

// Create registry with all rules
let registry = RuleRegistry::new();

// Get rules for a preset
let enabled: HashSet<String> = registry.get_preset_rules(Preset::Recommended);

// Parse PHP code (using mago-syntax)
let arena = bumpalo::Bump::new();
let file_id = mago_database::file::FileId::new("file.php");
let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);

// Run rules
let edits = registry.check_all(program, source, &enabled);

// Apply edits
let new_source = rustor_core::apply_edits(source, &edits)?;
```

## Rules

### Performance Rules

| Rule | Description | PHP Version |
|------|-------------|-------------|
| `array_push` | `array_push($a, $v)` → `$a[] = $v` | Any |
| `sizeof` | `sizeof()` → `count()` | Any |
| `pow_to_operator` | `pow($x, $n)` → `$x ** $n` | 5.6+ |
| `type_cast` | `strval($x)` → `(string) $x` | Any |
| `array_key_first_last` | `array_keys($a)[0]` → `array_key_first($a)` | 7.3+ |

### Modernization Rules

| Rule | Description | PHP Version |
|------|-------------|-------------|
| `array_syntax` | `array()` → `[]` | 5.4+ |
| `list_short_syntax` | `list($a, $b)` → `[$a, $b]` | 7.1+ |
| `isset_coalesce` | `isset($x) ? $x : $d` → `$x ?? $d` | 7.0+ |
| `empty_coalesce` | `empty($x) ? $d : $x` → `$x ?: $d` | 5.3+ |
| `assign_coalesce` | `$x = $x ?? $d` → `$x ??= $d` | 7.4+ |
| `null_safe_operator` | `$x !== null ? $x->y : null` → `$x?->y` | 8.0+ |
| `string_contains` | `strpos($s, $n) !== false` → `str_contains($s, $n)` | 8.0+ |
| `string_starts_ends` | `substr($s, 0, 1) === '/'` → `str_starts_with($s, '/')` | 8.0+ |
| `match_expression` | Simple switch → match | 8.0+ |
| `get_class_this` | `get_class($x)` → `$x::class` | 8.0+ |
| `first_class_callables` | `Closure::fromCallable('fn')` → `fn(...)` | 8.1+ |
| `constructor_promotion` | Property promotion (framework) | 8.0+ |
| `readonly_properties` | Add readonly (framework) | 8.1+ |

### Simplification Rules

| Rule | Description |
|------|-------------|
| `is_null` | `is_null($x)` → `$x === null` |
| `join_to_implode` | `join()` → `implode()` |
| `sprintf_positional` | Simple `sprintf()` → interpolation |

### Compatibility Rules

| Rule | Description | PHP Version |
|------|-------------|-------------|
| `class_constructor` | Legacy `ClassName()` → `__construct()` | 7.0+ |
| `implode_order` | Fix deprecated argument order | 7.4+ |

## Presets

```rust
use rustor_rules::Preset;

Preset::Recommended  // Safe, widely-applicable rules
Preset::Performance  // Performance optimizations
Preset::Modernize    // Syntax modernization
Preset::All          // All available rules
```

## Rule Trait

Implement to create new rules:

```rust
use rustor_rules::{Rule, Category, PhpVersion};

pub struct MyRule;

impl Rule for MyRule {
    fn name(&self) -> &'static str { "my_rule" }
    fn description(&self) -> &'static str { "Description" }
    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> { ... }
    fn category(&self) -> Category { Category::Simplification }
    fn min_php_version(&self) -> Option<PhpVersion> { None }
}
```

## Configurable Rules

Some rules support configuration:

```rust
use rustor_rules::{ConfigurableRule, ConfigValue};
use std::collections::HashMap;

let mut config = HashMap::new();
config.insert("loose_comparison".to_string(), ConfigValue::Bool(true));

let rule = StringContainsRule::with_config(&config);
```

## Testing

```bash
# All rule tests
cargo test -p rustor-rules

# Specific rule
cargo test -p rustor-rules is_null

# With output
cargo test -p rustor-rules -- --nocapture
```

## See Also

- [Rules Reference](../../docs/rules.md) - Detailed rule documentation
- [Development Guide](../../docs/development.md) - Adding new rules
- [rustor-core](../rustor-core/README.md) - Core types
