# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rustor is a PHP refactoring tool written in Rust with 44 refactoring rules and 55 formatting fixers. It uses format-preserving edits via span-based transformations.

## Build & Test Commands

```bash
# Build
cargo build --workspace              # Debug build
cargo build --release                # Release build (LTO, strip)

# Test
cargo test --workspace               # All tests
cargo test -p rustor-rules           # Single crate
cargo test -p rustor-rules is_null   # Single test
cargo test -- --nocapture            # With output

# Lint & Format
cargo fmt --all                      # Format code
cargo clippy --workspace             # Lint checks

# Run
cargo run -p rustor-cli -- src/      # Run on PHP files
```

## Workspace Architecture

Six crates with clear dependencies:

```
rustor-core      # Edit struct, Visitor trait (minimal, no deps on mago)
    ↑
rustor-rules     # 44 refactoring rules, RuleRegistry, Preset enum
    ↑
rustor-analyze   # PHPStan-compatible static analysis, NEON parsing
    ↑
rustor-fixer     # 55 PHP-CS-Fixer compatible formatters
    ↑
rustor-cli       # Binary: CLI, LSP server, config, caching, watch mode
```

`rustor-rector-import` - Standalone tool for importing Rector rules (binary: `rustor-import-rector`)

## Key Dependencies

- **mago-syntax** (1.0) - PHP parser/AST
- **mago-span** (1.0) - Source positions for format-preserving edits
- **rayon** - Parallel file processing
- **clap** (4.5) - CLI argument parsing

## Adding a New Refactoring Rule

1. Create `crates/rustor-rules/src/rule_name.rs`:

```rust
use mago_syntax::ast::*;
use rustor_core::{Edit, Visitor};

pub fn check_rule<'a>(program: &Program<'a>, source: &str) -> Vec<Edit> {
    let mut visitor = RuleVisitor { source, edits: Vec::new() };
    visitor.visit_program(program, source);
    visitor.edits
}

struct RuleVisitor<'s> {
    source: &'s str,
    edits: Vec<Edit>,
}

impl<'a, 's> Visitor<'a> for RuleVisitor<'s> {
    fn visit_expression(&mut self, expr: &Expression<'a>, _source: &str) -> bool {
        // Match patterns and create edits
        // Use span to extract text: &self.source[span.start.offset as usize..span.end.offset as usize]
        true // continue visiting children
    }
}
```

2. Register in `crates/rustor-rules/src/registry.rs`:
   - Add to `Rule` trait implementations
   - Include in appropriate preset (recommended, performance, modernize, all)

3. Add tests inline with `#[cfg(test)]` module

## Span-Based Editing Pattern

```rust
// Extract source text from a span
let span = node.span();
let text = &source[span.start.offset as usize..span.end.offset as usize];

// Create an edit
Edit::new(span, replacement_string, message)
```

## Configuration Files

- `.rustor.toml` - Main project configuration (PHP version, rules, paths)
- `.php-cs-fixer.php` - PHP-CS-Fixer compatible config for fixers

## Output Formats

text, diff, json, sarif, html, checkstyle, github (for GitHub Actions annotations)

## Permissions -- claude code permissions
    # always allow to execute rustor locally even to perform analysis on other directories, always use the phpstan

## Comparing rustor to phpstan
    # always run the following commands
   -  `cd /Users/borisyv/code/payjoy_www && echo "=== Running Rustor ===" && time /Users/borisyv/RustProjects/rustor/target/release/rustor analyze  -c phpstan.neon.dist --format=table --phpstan_compat`
   -  `cd /Users/borisyv/code/payjoy_www && echo "=== Running PHPStan with increased memory ===" && time ./libs/vendor/bin/phpstan analyze --configuration=phpstan.neon --memory-limit=-1 -format=table`
