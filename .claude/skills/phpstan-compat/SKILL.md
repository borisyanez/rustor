---
name: phpstan-compat
description: Achieve 100% compatibility between rustor analyze and PHPStan. Use when comparing outputs, adding missing checks, fixing discrepancies, or tracking compatibility progress. Provides methodology for systematic comparison and implementation guidance.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
  - Write
  - Edit
  - Task
---

# PHPStan Compatibility Skill

Systematic approach to achieving 100% compatibility between rustor's static analysis and PHPStan.

## Overview

This skill guides the process of:
1. Measuring current compatibility status
2. Identifying discrepancies between rustor and PHPStan
3. Implementing missing checks
4. Verifying fixes don't introduce regressions

## Compatibility Goals

Target: **100% error parity** - rustor should report the same errors as PHPStan for any given codebase at the same analysis level.

### What "100% Compatible" Means

- Same error identifiers (e.g., `function.notFound`, `class.notFound`)
- Same error messages (or semantically equivalent)
- Same file locations and line numbers
- Same severity levels
- Same behavior with baselines, ignores, and excludes

## Workflow

### Step 1: Measure Current State

Run the comparison commands from CLAUDE.md:

```bash
# From the payjoy_www directory
cd /Users/borisyv/code/payjoy_www

# Run PHPStan
echo "=== Running PHPStan ===" && \
time ./libs/vendor/bin/phpstan analyze \
  --configuration=phpstan.neon \
  --memory-limit=-1 \
  --error-format=json > /tmp/phpstan-output.json 2>&1

# Run Rustor
echo "=== Running Rustor ===" && \
time /Users/borisyv/RustProjects/rustor/target/release/rustor analyze \
  -c phpstan.neon.dist \
  --format=json \
  --phpstan_compat > /tmp/rustor-output.json 2>&1
```

### Step 2: Compare Outputs

Use JSON comparison to identify differences:

```bash
# Extract error counts
jq '.totals' /tmp/phpstan-output.json
jq '.totals' /tmp/rustor-output.json

# Find errors in PHPStan but not in rustor (false negatives)
jq -r '.files | to_entries[] | .value.messages[] | "\(.file):\(.line):\(.message)"' /tmp/phpstan-output.json | sort > /tmp/phpstan-errors.txt
jq -r '.files | to_entries[] | .value.messages[] | "\(.file):\(.line):\(.message)"' /tmp/rustor-output.json | sort > /tmp/rustor-errors.txt

# Errors in PHPStan but missing from rustor
comm -23 /tmp/phpstan-errors.txt /tmp/rustor-errors.txt > /tmp/missing-in-rustor.txt

# Errors in rustor but not in PHPStan (false positives)
comm -13 /tmp/phpstan-errors.txt /tmp/rustor-errors.txt > /tmp/extra-in-rustor.txt
```

### Step 3: Categorize Discrepancies

Group differences by type:

| Category | Priority | Description |
|----------|----------|-------------|
| Missing Check | High | PHPStan reports error, rustor doesn't |
| False Positive | High | Rustor reports error, PHPStan doesn't |
| Wrong Message | Medium | Same issue, different message format |
| Wrong Line | Low | Same issue, off-by-one line number |
| Config Handling | Medium | Different interpretation of neon config |

### Step 4: Implement Fixes

For each discrepancy type:

#### Missing Check (False Negative)

1. Identify which PHPStan check produces the error
2. Check if rustor has an equivalent check at the correct level
3. If missing, implement the check in `crates/rustor-analyze/src/checks/levelN/`
4. Register in `CheckRegistry::with_builtin_checks()`

#### False Positive

1. Identify why rustor reports the error incorrectly
2. Common causes:
   - Missing symbol resolution (function/class not found in autoload)
   - Incorrect type inference
   - Missing PHPDoc parsing
3. Fix the underlying issue in symbol table or type system

### Step 5: Verify Fix

After each fix:

```bash
# Rebuild
cargo build --release -p rustor-cli

# Re-run comparison
cd /Users/borisyv/code/payjoy_www && \
/Users/borisyv/RustProjects/rustor/target/release/rustor analyze \
  -c phpstan.neon.dist --format=json --phpstan_compat > /tmp/rustor-new.json

# Compare counts
echo "Before: $(wc -l < /tmp/missing-in-rustor.txt) missing"
echo "After: $(comm -23 /tmp/phpstan-errors.txt <(jq -r '...' /tmp/rustor-new.json | sort) | wc -l) missing"
```

## Check Implementation Guide

### Adding a New Check

Location: `crates/rustor-analyze/src/checks/levelN/`

```rust
use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_syntax::ast::*;

pub struct MyNewCheck;

impl Check for MyNewCheck {
    fn id(&self) -> &'static str {
        "identifier.subtype"  // Match PHPStan's identifier
    }

    fn description(&self) -> &'static str {
        "Description of what this check finds"
    }

    fn level(&self) -> u8 {
        0  // PHPStan level at which this activates
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = MyCheckVisitor {
            ctx,
            issues: Vec::new(),
        };
        visitor.visit_program(program);
        visitor.issues
    }
}
```

### PHPStan Level Reference

| Level | Checks Added |
|-------|--------------|
| 0 | Basic errors: undefined functions, classes, constants |
| 1 | Possibly undefined variables, magic methods |
| 2 | Unknown methods on known types, property access |
| 3 | Return types, property types |
| 4 | Dead code, unused results, always-false conditions |
| 5 | Argument types |
| 6 | Missing typehints |
| 7 | Union types |
| 8 | Nullable types |
| 9 | Explicit mixed |

## Common Compatibility Issues

### 1. Symbol Resolution

PHPStan uses Composer's autoload; rustor must replicate this:
- Classmap loading from `vendor/composer/autoload_classmap.php`
- PSR-4 namespace resolution
- Include/require file scanning

### 2. Type Inference

PHPStan has sophisticated type inference. For compatibility:
- Parse PHPDoc `@var`, `@param`, `@return` annotations
- Track variable types through control flow
- Handle union types, generics, conditional types

### 3. Configuration Parsing

Ensure NEON config handling matches PHPStan:
- `ignoreErrors` patterns
- `excludePaths` handling
- `includes` file resolution
- `parameters` merging

### 4. Error Message Format

Match PHPStan's message format exactly:
- Use same terminology ("Function X not found" vs "Undefined function X")
- Include same context in messages
- Use same identifier format

## Progress Tracking

Create a tracking file to monitor progress:

```markdown
# PHPStan Compatibility Progress

## Current Status
- Total PHPStan errors: X
- Total Rustor errors: Y
- Missing (false negatives): Z
- Extra (false positives): W
- Compatibility: (X-Z)/X * 100 = N%

## Recent Changes
- [date] Implemented check X, fixed N errors
- [date] Fixed symbol resolution for Y, fixed M errors

## Known Issues
- [ ] Missing: undefined constant in class context
- [ ] False positive: functions from stubs not recognized
```

## Testing Strategy

### Unit Tests

Each check should have unit tests in its module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reports_same_as_phpstan() {
        let source = r#"<?php
        unknownFunction();
        "#;
        // Test that rustor reports the same error PHPStan would
    }
}
```

### Integration Tests

Compare against a reference PHPStan output:

```bash
# Generate reference
phpstan analyze tests/fixtures/ --error-format=json > tests/fixtures/expected.json

# Compare rustor output
rustor analyze tests/fixtures/ --format=json | diff - tests/fixtures/expected.json
```

## Quick Reference Commands

```bash
# Build release
cargo build --release -p rustor-cli

# Run on test codebase
cd /Users/borisyv/code/payjoy_www && \
/Users/borisyv/RustProjects/rustor/target/release/rustor analyze \
  -c phpstan.neon.dist --format=table --phpstan_compat

# Compare error counts
echo "PHPStan:" && ./libs/vendor/bin/phpstan analyze -c phpstan.neon --error-format=json | jq '.totals'
echo "Rustor:" && /Users/borisyv/RustProjects/rustor/target/release/rustor analyze -c phpstan.neon.dist --format=json --phpstan_compat | jq '.totals'

# Run specific check test
cargo test -p rustor-analyze undefined_function -- --nocapture
```

## Execution Checklist

When working on compatibility:

1. [ ] Run comparison to get current state
2. [ ] Identify top discrepancy category
3. [ ] Pick specific error to fix
4. [ ] Implement/fix check
5. [ ] Add unit test
6. [ ] Rebuild and re-compare
7. [ ] Update progress tracking
8. [ ] Repeat
