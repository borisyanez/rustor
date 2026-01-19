# Rustor Rule Development Tutorial

This tutorial teaches you how to create refactoring rules for rustor. We'll cover three complexity levels:

1. **Simple rules** - Function aliases (e.g., `join` → `implode`)
2. **Medium rules** - Function-to-operator transformations (e.g., `is_null($x)` → `$x === null`)
3. **Complex rules** - Multi-pattern matching (e.g., ternary to `get_debug_type()`)

## Prerequisites

- Basic Rust knowledge
- Understanding of PHP syntax
- Familiarity with AST (Abstract Syntax Tree) concepts

## Architecture Overview

Each rule follows this structure:

```
src/rule_name.rs
├── check_rule_name()        # Public entry point
├── RuleNameVisitor          # AST visitor struct
├── impl Visitor for ...     # Visit implementation
├── helper functions         # Pattern matching helpers
├── RuleNameRule             # Rule trait implementation
└── #[cfg(test)] mod tests   # Test module
```

### Key Types

- `mago_syntax::ast::*` - PHP AST types (`Expression`, `FuncCall`, `Conditional`, etc.)
- `mago_span::Span` - Source positions for format-preserving edits
- `rustor_core::Edit` - Replacement operations
- `rustor_core::Visitor` - AST traversal trait

---

## Level 1: Simple Function Alias Rule

Let's examine `join_to_implode.rs` - a rule that replaces `join()` with `implode()`.

### Transformation

```php
// Before
join(',', $arr);

// After
implode(',', $arr);
```

### Implementation

```rust
//! Rule: Convert join() to implode()
//!
//! join() is an alias for implode(). Using implode() is more common.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

/// Check a parsed PHP program for join() calls
pub fn check_join_to_implode<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = JoinToImplodeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct JoinToImplodeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for JoinToImplodeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match function calls
        if let Expression::Call(Call::Function(func_call)) = expr {
            // Get function name
            if let Expression::Identifier(ident) = func_call.function {
                let name_span = ident.span();
                let name = &self.source[name_span.start.offset as usize..name_span.end.offset as usize];

                // Check if it's join() (case-insensitive)
                if name.eq_ignore_ascii_case("join") {
                    // Get the full argument list to preserve
                    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

                    // Extract argument text from source
                    let args_text = if !args.is_empty() {
                        let first_span = args.first().unwrap().span();
                        let last_span = args.last().unwrap().span();
                        &self.source[first_span.start.offset as usize..last_span.end.offset as usize]
                    } else {
                        ""
                    };

                    // Create replacement
                    let replacement = format!("implode({})", args_text);

                    self.edits.push(Edit::new(
                        expr.span(),
                        replacement,
                        "Replace join() with implode() - join is an alias",
                    ));

                    return false; // Don't traverse children
                }
            }
        }
        true // Continue traversal
    }
}

// Rule trait implementation
use crate::registry::{Category, Rule};

pub struct JoinToImplodeRule;

impl Rule for JoinToImplodeRule {
    fn name(&self) -> &'static str {
        "join_to_implode"
    }

    fn description(&self) -> &'static str {
        "Convert join() to implode()"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_join_to_implode(program, source)
    }

    fn category(&self) -> Category {
        Category::Simplification
    }
}
```

### Key Concepts

1. **AST Pattern Matching**: `Expression::Call(Call::Function(func_call))` matches function calls
2. **Extracting Names**: Use spans to get the actual source text
3. **Case Insensitivity**: PHP function names are case-insensitive
4. **Preserving Arguments**: Extract argument text from source to preserve formatting

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_join_to_implode(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_join_to_implode() {
        let source = "<?php join(',', $arr);";
        assert_eq!(transform(source), "<?php implode(',', $arr);");
    }

    #[test]
    fn test_skip_implode() {
        // Should not match implode()
        let source = "<?php implode(',', $arr);";
        assert_eq!(check_php(source).len(), 0);
    }
}
```

---

## Level 2: Function-to-Operator Transformation

Let's examine `is_null.rs` - a rule that transforms function calls to operator expressions.

### Transformation

```php
// Before
is_null($x)
!is_null($x)

// After
$x === null
$x !== null
```

### Implementation

```rust
//! Rule: Convert is_null($x) to $x === null
//!
//! This transformation improves performance by avoiding function call overhead.
//! Also handles negation: !is_null($x) → $x !== null

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

pub fn check_is_null<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = IsNullVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct IsNullVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for IsNullVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Handle !is_null($x) → $x !== null
        if let Expression::UnaryPrefix(unary) = expr {
            if let UnaryPrefixOperator::Not(_) = &unary.operator {
                if let Some(replacement) = try_transform_is_null(&unary.operand, self.source, true) {
                    self.edits.push(Edit::new(
                        expr.span(),  // Use the full expression span (including !)
                        replacement,
                        "Replace !is_null() with !== null for better performance",
                    ));
                    return false;
                }
            }
        }

        // Handle is_null($x) → $x === null
        if let Some(replacement) = try_transform_is_null(expr, self.source, false) {
            self.edits.push(Edit::new(
                expr.span(),
                replacement,
                "Replace is_null() with === null for better performance",
            ));
            return false;
        }

        true
    }
}

/// Try to transform an is_null() call, returning the replacement string if successful
fn try_transform_is_null(expr: &Expression<'_>, source: &str, negated: bool) -> Option<String> {
    // Match: Expression::Call(Call::Function(func_call))
    if let Expression::Call(Call::Function(func_call)) = expr {
        // Check function name is "is_null"
        if let Expression::Identifier(ident) = func_call.function {
            let name_span = ident.span();
            let name = &source[name_span.start.offset as usize..name_span.end.offset as usize];

            if name.eq_ignore_ascii_case("is_null") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();

                // is_null takes exactly 1 argument
                if args.len() == 1 {
                    // Skip if argument contains an assignment (precedence issues)
                    if matches!(args[0].value(), Expression::Assignment(_)) {
                        return None;
                    }

                    let arg_span = args[0].span();
                    let arg_code = &source[arg_span.start.offset as usize..arg_span.end.offset as usize];

                    let operator = if negated { "!==" } else { "===" };
                    return Some(format!("{} {} null", arg_code, operator));
                }
            }
        }
    }
    None
}
```

### Key Concepts

1. **Negation Handling**: Check for `UnaryPrefixOperator::Not` wrapping the call
2. **Argument Validation**: Verify argument count before transformation
3. **Edge Case Prevention**: Skip assignments to avoid precedence issues
4. **Span Selection**: Use the outer expression span to include the `!` operator

### Important Edge Cases

```rust
#[test]
fn test_skip_assignment_inside_is_null() {
    // Skip: is_null($x = foo()) would become $x = foo() === null
    // which has wrong precedence (assigns boolean to $x)
    let source = "<?php if (is_null($result = getValue())) {}";
    let edits = check_php(source);
    assert_eq!(edits.len(), 0);
}
```

---

## Level 3: Complex Multi-Pattern Matching

The `get_debug_type.rs` rule demonstrates complex ternary pattern matching.

### Transformation

```php
// Before
is_object($value) ? get_class($value) : gettype($value)
is_object($value) ? $value::class : gettype($value)

// After
get_debug_type($value)
```

### Implementation Strategy

This rule requires matching a ternary with specific functions in each position and verifying all positions use the same variable.

```rust
//! Rule: Convert `is_object($x) ? get_class($x) : gettype($x)` to `get_debug_type($x)`
//!
//! PHP 8.0 introduced `get_debug_type()` which returns a more useful type string.

use mago_span::HasSpan;
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

pub fn check_get_debug_type<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = GetDebugTypeVisitor {
        source,
        edits: Vec::new(),
    };
    visitor.visit_program(program, source);
    visitor.edits
}

struct GetDebugTypeVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for GetDebugTypeVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match ternary expression (Conditional in mago-syntax)
        if let Expression::Conditional(cond) = expr {
            if let Some(replacement) = try_transform_get_debug_type(cond, self.source) {
                self.edits.push(Edit::new(
                    expr.span(),
                    replacement,
                    "Replace type-checking ternary with get_debug_type() (PHP 8.0+)",
                ));
                return false;
            }
        }
        true
    }
}

/// Try to transform a conditional to get_debug_type()
fn try_transform_get_debug_type(cond: &Conditional<'_>, source: &str) -> Option<String> {
    // Must be a full ternary (not short ternary like $a ?: $b)
    let then_expr = cond.then.as_ref()?;

    // Step 1: Check condition is is_object($var)
    let condition_var = extract_is_object_var(cond.condition, source)?;

    // Step 2: Check then-branch is get_class($var) or $var::class
    let then_var = extract_get_class_or_class_const(then_expr, source)?;

    // Step 3: Check else-branch is gettype($var)
    let else_var = extract_gettype_var(cond.r#else, source)?;

    // Step 4: All variables must be identical
    if condition_var != then_var || condition_var != else_var {
        return None;
    }

    // All checks passed - create the replacement
    Some(format!("get_debug_type({})", condition_var))
}

/// Extract variable from is_object($var) call
fn extract_is_object_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name = get_span_text(ident.span(), source);
            if name.eq_ignore_ascii_case("is_object") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                if args.len() == 1 {
                    return Some(get_span_text(args[0].span(), source).to_string());
                }
            }
        }
    }
    None
}

/// Extract variable from get_class($var) or $var::class
fn extract_get_class_or_class_const<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    match expr {
        // get_class($var)
        Expression::Call(Call::Function(func_call)) => {
            if let Expression::Identifier(ident) = func_call.function {
                let name = get_span_text(ident.span(), source);
                if name.eq_ignore_ascii_case("get_class") {
                    let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                    if args.len() == 1 {
                        return Some(get_span_text(args[0].span(), source).to_string());
                    }
                }
            }
            None
        }
        // $var::class
        Expression::Access(Access::ClassConstant(cc)) => {
            // Check if accessing ::class constant
            let const_name = match &cc.constant {
                ClassLikeConstantSelector::Identifier(ident) => {
                    get_span_text(ident.span(), source)
                }
                _ => return None,
            };

            if const_name == "class" {
                let class_text = get_span_text(cc.class.span(), source);
                // Only match variable expressions like $var
                if class_text.starts_with('$') {
                    return Some(class_text.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract variable from gettype($var) call
fn extract_gettype_var<'a>(expr: &Expression<'a>, source: &str) -> Option<String> {
    if let Expression::Call(Call::Function(func_call)) = expr {
        if let Expression::Identifier(ident) = func_call.function {
            let name = get_span_text(ident.span(), source);
            if name.eq_ignore_ascii_case("gettype") {
                let args: Vec<_> = func_call.argument_list.arguments.iter().collect();
                if args.len() == 1 {
                    return Some(get_span_text(args[0].span(), source).to_string());
                }
            }
        }
    }
    None
}

/// Helper to get source text from a span
fn get_span_text(span: mago_span::Span, source: &str) -> &str {
    &source[span.start.offset as usize..span.end.offset as usize]
}
```

### Key Concepts

1. **Conditional Type**: Ternaries are `Expression::Conditional` in mago-syntax
2. **Multiple Extraction Helpers**: Each pattern position has its own extractor
3. **Variable Consistency Check**: All three positions must reference the same variable
4. **Alternative Syntax**: Handle both `get_class($x)` and `$x::class` syntaxes
5. **Class Constant Access**: Match `$var::class` with `Access::ClassConstant`

### Rule Trait with PHP Version

```rust
use crate::registry::{Category, PhpVersion, Rule};

pub struct GetDebugTypeRule;

impl Rule for GetDebugTypeRule {
    fn name(&self) -> &'static str {
        "get_debug_type"
    }

    fn description(&self) -> &'static str {
        "Convert is_object() ? get_class() : gettype() to get_debug_type() (PHP 8.0+)"
    }

    fn check<'a>(&self, program: &Program<'a>, source: &str) -> Vec<Edit> {
        check_get_debug_type(program, source)
    }

    fn category(&self) -> Category {
        Category::Modernization
    }

    fn min_php_version(&self) -> Option<PhpVersion> {
        Some(PhpVersion::Php80)  // get_debug_type() requires PHP 8.0
    }
}
```

---

## Registering Your Rule

After creating your rule, register it in two places:

### 1. Add module to `lib.rs`

```rust
// In crates/rustor-rules/src/lib.rs
pub mod get_debug_type;  // Add module

// Re-export check function
pub use get_debug_type::check_get_debug_type;
```

### 2. Register in `registry.rs`

```rust
// In RuleRegistry::new_with_config()
registry.register(Box::new(super::get_debug_type::GetDebugTypeRule));

// Add to appropriate preset
Preset::Modernize => &[
    // ...
    "get_debug_type",
    // ...
],
```

---

## Common AST Patterns

### Function Calls

```rust
Expression::Call(Call::Function(func_call)) => {
    // func_call.function - the function name/expression
    // func_call.argument_list.arguments - the arguments
}
```

### Method Calls

```rust
Expression::Access(Access::Property(prop_access)) => {
    // prop_access.object - the object
    // prop_access.property - the property name
}

Expression::Call(Call::Method(method_call)) => {
    // method_call.object - the object
    // method_call.method - the method name
    // method_call.argument_list - arguments
}
```

### Ternary/Conditional

```rust
Expression::Conditional(cond) => {
    // cond.condition - the condition expression
    // cond.then - Option<Expression> (None for short ternary ?:)
    // cond.r#else - the else expression
}
```

### Class Constant Access

```rust
Expression::Access(Access::ClassConstant(cc)) => {
    // cc.class - the class expression
    // cc.constant - ClassLikeConstantSelector
}
```

### Unary Operations

```rust
Expression::UnaryPrefix(unary) => {
    // unary.operator - UnaryPrefixOperator (Not, Minus, etc.)
    // unary.operand - the operand expression
}
```

---

## Testing Best Practices

1. **Test basic transformation**: The happy path
2. **Test case insensitivity**: PHP function names are case-insensitive
3. **Test skip cases**: Ensure similar but different patterns are not matched
4. **Test nested contexts**: In arrays, methods, closures, etc.
5. **Test edge cases**: Whitespace, parentheses, complex expressions
6. **Test multiple occurrences**: Multiple matches in one file

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use rustor_core::apply_edits;

    fn check_php(source: &str) -> Vec<Edit> {
        let arena = Bump::new();
        let file_id = FileId::new("test.php");
        let (program, _) = mago_syntax::parser::parse_file_content(&arena, file_id, source);
        check_your_rule(program, source)
    }

    fn transform(source: &str) -> String {
        let edits = check_php(source);
        apply_edits(source, &edits).unwrap()
    }

    #[test]
    fn test_basic() {
        let source = "<?php your_pattern();";
        assert_eq!(transform(source), "<?php expected_result();");
    }

    #[test]
    fn test_skip_not_matching() {
        let source = "<?php different_pattern();";
        assert_eq!(check_php(source).len(), 0);
    }
}
```

---

## YAML Rules Alternative

For simple patterns, you can define rules in YAML without writing Rust:

```yaml
# rules/sizeof_to_count.yaml
name: sizeof_to_count
description: "Replace sizeof() with count()"
category: code_quality

match:
  node: FuncCall
  name: sizeof
  args:
    - capture: $arr

replace: "count($arr)"

tests:
  - input: "sizeof($array)"
    output: "count($array)"
```

Load YAML rules with:
```rust
registry.load_yaml_rules_from_dir(&Path::new("rules"))?;
```

---

## Summary

| Level | Example | Key Pattern |
|-------|---------|-------------|
| Simple | `join_to_implode` | Function name replacement |
| Medium | `is_null` | Function to operator, handle negation |
| Complex | `get_debug_type` | Multi-pattern ternary matching |

When creating a new rule:
1. Identify the transformation pattern
2. Choose the appropriate AST types to match
3. Create helper functions for each sub-pattern
4. Implement the visitor and rule trait
5. Write comprehensive tests
6. Register in lib.rs and registry.rs
