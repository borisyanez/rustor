# Development Guide

This guide covers rustor's architecture, how to add new rules, and contribution guidelines.

## Architecture Overview

Rustor is organized as a Cargo workspace with three crates:

```
rustor/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── rustor-core/        # Core types: Edit, apply_edits, Visitor
│   ├── rustor-rules/       # All 23 refactoring rules
│   └── rustor-cli/         # CLI application
└── docs/                   # Documentation
```

### Crate Dependencies

```
rustor-cli
    ├── rustor-rules
    │   └── rustor-core
    └── rustor-core
```

---

## Core Concepts

### Edit

The fundamental output type. Represents a text replacement:

```rust
// crates/rustor-core/src/lib.rs
pub struct Edit {
    pub span: Span,          // Start and end position
    pub replacement: String, // New text
    pub message: String,     // Description of the change
}
```

### Visitor Trait

Traverses the PHP AST:

```rust
// crates/rustor-core/src/lib.rs
pub trait Visitor<'a> {
    fn visit_program(&mut self, program: &Program<'a>, source: &str) { ... }
    fn visit_statement(&mut self, stmt: &Statement<'a>, source: &str) -> bool { true }
    fn visit_expression(&mut self, expr: &Expression<'a>, source: &str) -> bool { true }
}
```

### Rule Trait

Interface for all refactoring rules:

```rust
// crates/rustor-rules/src/registry.rs
pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit>;
    fn min_php_version(&self) -> Option<PhpVersion> { None }
    fn category(&self) -> Category { Category::Simplification }
}
```

---

## Adding a New Rule

### Step 1: Create the Rule File

Create `crates/rustor-rules/src/my_rule.rs`:

```rust
//! Rule: Convert foo() to bar()
//!
//! Example:
//! ```php
//! // Before
//! foo($x);
//!
//! // After
//! bar($x);
//! ```

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

use crate::registry::{Category, PhpVersion, Rule};

/// Check for foo() calls and suggest bar()
pub fn check_my_rule<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = MyRuleVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct MyRuleVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for MyRuleVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        self.check_expression(expr);
        true // Continue traversal
    }
}

impl<'s> MyRuleVisitor<'s> {
    fn check_expression(&mut self, expr: &Expression<'_>) {
        if let Expression::Call(call) = expr {
            if let Call::Function(func_call) = call {
                if let Expression::Identifier(ident) = func_call.function {
                    let name_span = ident.span();
                    let name = &self.source[name_span.start.offset as usize..name_span.end.offset as usize];

                    if name.eq_ignore_ascii_case("foo") {
                        self.edits.push(Edit::new(
                            name_span,
                            "bar".to_string(),
                            "Convert foo() to bar()",
                        ));
                    }
                }
            }
        }
    }
}

pub struct MyRule;

impl Rule for MyRule {
    fn name(&self) -> &'static str {
        "my_rule"
    }

    fn description(&self) -> &'static str {
        "Convert foo() to bar()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_my_rule(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php74) // Optional: set minimum PHP version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_my_rule(program, source)
    }

    #[test]
    fn test_simple_conversion() {
        let source = r#"<?php foo($x);"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "bar");
    }

    #[test]
    fn test_skip_bar() {
        let source = r#"<?php bar($x);"#;
        let edits = check_php(source);
        assert_eq!(edits.len(), 0);
    }
}
```

### Step 2: Register the Rule

Add to `crates/rustor-rules/src/lib.rs`:

```rust
pub mod my_rule;

// In re-exports section:
pub use my_rule::check_my_rule;
```

Add to `crates/rustor-rules/src/registry.rs`:

```rust
// In RuleRegistry::new_with_config():
registry.register(Box::new(super::my_rule::MyRule));

// In Preset::rules() if applicable:
Preset::Recommended => &[
    // ... existing rules
    "my_rule",
],
```

### Step 3: Run Tests

```bash
cargo test -p rustor-rules my_rule
cargo test --workspace
```

### Step 4: Test on Real Code

```bash
cargo build --release
./target/release/rustor /tmp/test-project --rule my_rule --dry-run
```

---

## Project Structure

### rustor-core

Minimal core library with no dependencies on PHP parsing:

```
crates/rustor-core/
├── Cargo.toml
└── src/
    ├── lib.rs           # Edit struct, apply_edits()
    └── visitor.rs       # Visitor trait
```

Key exports:
- `Edit` - Text replacement with span and message
- `apply_edits()` - Apply multiple edits to source
- `Visitor` - AST traversal trait

### rustor-rules

All refactoring rules:

```
crates/rustor-rules/
├── Cargo.toml
└── src/
    ├── lib.rs           # Module declarations, re-exports
    ├── registry.rs      # Rule trait, RuleRegistry, presets
    ├── array_push.rs    # Individual rule
    ├── is_null.rs
    └── ...              # 23 rule files
```

Key exports:
- `Rule` trait
- `RuleRegistry` - Rule management
- `Preset`, `Category`, `PhpVersion` - Enums
- Individual rule check functions

### rustor-cli

Command-line application:

```
crates/rustor-cli/
├── Cargo.toml
└── src/
    ├── main.rs          # CLI entry point, argument parsing
    ├── process.rs       # File processing logic
    ├── output.rs        # Output formatting (text, json, sarif, etc.)
    ├── config.rs        # .rustor.toml parsing
    ├── cache.rs         # File caching
    ├── watch.rs         # Watch mode
    ├── git.rs           # Git integration (--staged, --since)
    ├── baseline.rs      # Baseline support
    ├── ignore.rs        # Inline ignore comments
    ├── backup.rs        # Backup/restore functionality
    └── lsp.rs           # LSP server
```

---

## Testing

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p rustor-rules

# Specific rule
cargo test -p rustor-rules is_null

# With output
cargo test -p rustor-rules -- --nocapture
```

### Test Structure

Each rule should have tests for:

1. **Basic transformation** - The happy path
2. **Edge cases** - Nested expressions, complex syntax
3. **Skip cases** - When the rule should NOT apply
4. **Multiple occurrences** - Finding all instances
5. **Context variations** - In functions, classes, namespaces

Example test structure:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn check_php(source: &str) -> Vec<Edit> { ... }
    fn transform(source: &str) -> String { ... }

    // ==================== Basic Tests ====================

    #[test]
    fn test_simple_case() { ... }

    #[test]
    fn test_with_variables() { ... }

    // ==================== Skip Tests ====================

    #[test]
    fn test_skip_different_function() { ... }

    #[test]
    fn test_skip_method_call() { ... }

    // ==================== Context Tests ====================

    #[test]
    fn test_in_function() { ... }

    #[test]
    fn test_in_class_method() { ... }

    #[test]
    fn test_deeply_nested() { ... }
}
```

---

## Code Style

### Rust Style

- Follow standard Rust conventions
- Use `rustfmt` for formatting
- Use `clippy` for linting

```bash
cargo fmt --all
cargo clippy --workspace
```

### Rule Naming

- Rule names: `snake_case` (e.g., `array_push`, `isset_coalesce`)
- Module files: same as rule name
- Struct names: `PascalCase` + `Rule` suffix (e.g., `ArrayPushRule`)

### Documentation

- Every rule file starts with module doc comment
- Include before/after examples
- Document any limitations or skip conditions

---

## Performance Considerations

### Parsing

- Use `bumpalo` arena allocator (provided by mago-syntax)
- Avoid cloning AST nodes when possible
- Extract text from source using spans, not string operations

### Rule Execution

- Rules are run in parallel across files (rayon)
- Each rule should be stateless
- Avoid expensive operations in hot paths

### Caching

- File content hash determines cache validity
- Rule set hash invalidates cache on rule changes
- Cache stored in `.rustor-cache/`

---

## Common Patterns

### Extracting Text from Span

```rust
let span = node.span();
let text = &source[span.start.offset as usize..span.end.offset as usize];
```

### Matching Function Calls

```rust
if let Expression::Call(Call::Function(func_call)) = expr {
    if let Expression::Identifier(ident) = func_call.function {
        let name = /* extract from span */;
        if name.eq_ignore_ascii_case("target_function") {
            // Process arguments...
        }
    }
}
```

### Handling Arguments

```rust
let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
if args.len() == 2 {
    let first_arg = args[0].value();
    let second_arg = args[1].value();
    // Process...
}
```

### Creating Edits

```rust
self.edits.push(Edit::new(
    node.span(),
    replacement_text,
    "Description of the change",
));
```

---

## Debugging

### Printing AST

```rust
// In a rule, temporarily add:
eprintln!("Expression: {:?}", expr);
```

### Running Single File

```bash
./target/release/rustor test.php --rule my_rule --format diff
```

### Verbose Output

```bash
./target/release/rustor src/ -v --rule my_rule
```

---

## Contributing

### Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Run `cargo test --workspace`
5. Run `cargo fmt --all && cargo clippy --workspace`
6. Submit PR with clear description

### Commit Messages

Follow conventional commits:

```
feat(rules): add my_new_rule for converting foo to bar

- Handles basic foo() calls
- Skips method calls
- 10 tests added
```

### Issue Reporting

Include:
- PHP code that triggers the issue
- Expected behavior
- Actual behavior
- Rustor version

---

## Release Process

1. Update version in `Cargo.toml` files
2. Update CHANGELOG.md
3. Create git tag: `git tag v0.3.0`
4. Build release: `cargo build --release`
5. Create GitHub release with binaries

---

## See Also

- [Rules Reference](rules.md) - Existing rule implementations
- [CLI Reference](cli.md) - Command-line options
- [mago-syntax documentation](https://docs.rs/mago-syntax) - PHP parser
