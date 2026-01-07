# Phase 0: Quick Start Implementation Guide

This guide walks you through the first proof-of-concept implementation.

## Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Useful tools
cargo install cargo-watch  # For auto-rebuild
cargo install just         # Task runner (optional)
```

## Step 1: Clone and Study Mago

```bash
# Clone mago to study its internals
git clone https://github.com/carthage-software/mago.git ~/projects/mago
cd ~/projects/mago

# Build to ensure everything works
cargo build --release

# Run on a test project to see it in action
./target/release/mago lint /path/to/php/project
```

### Key files to study:

```
crates/
├── mago-syntax/src/         # Parser implementation
│   ├── lexer/               # Tokenizer
│   ├── parser/              # Recursive descent parser
│   └── ast/                 # AST node definitions
│
├── mago-fixer/src/          # ⚠️ CRITICAL - Study this first!
│   ├── lib.rs               # Fix application logic
│   └── ...
│
├── mago-linter/src/         # How rules are structured
│   ├── plugin/              # Rule organization
│   └── rules/               # Individual lint rules
│
└── mago-span/src/           # Source position handling
    └── lib.rs
```

## Step 2: Create PoC Project

```bash
mkdir -p ~/projects/php-refactor-poc
cd ~/projects/php-refactor-poc
cargo init --name php-refactor-poc
```

### Cargo.toml

```toml
[package]
name = "php-refactor-poc"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core mago crates
mago-syntax = { git = "https://github.com/carthage-software/mago", branch = "main" }
mago-source = { git = "https://github.com/carthage-software/mago", branch = "main" }
mago-span = { git = "https://github.com/carthage-software/mago", branch = "main" }
mago-interner = { git = "https://github.com/carthage-software/mago", branch = "main" }

# Utilities
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }
colored = "2.0"

[dev-dependencies]
pretty_assertions = "1.4"
```

## Step 3: Minimal Implementation

### src/main.rs

```rust
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod rules;
mod editor;

#[derive(Parser)]
#[command(name = "php-refactor-poc")]
#[command(about = "A proof-of-concept PHP refactoring tool")]
struct Cli {
    /// Files or directories to process
    #[arg(required = true)]
    paths: Vec<PathBuf>,
    
    /// Show changes without applying them
    #[arg(long, short = 'n')]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    for path in &cli.paths {
        if path.is_file() {
            process_file(path, cli.dry_run)?;
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "php"))
            {
                process_file(entry.path(), cli.dry_run)?;
            }
        }
    }
    
    Ok(())
}

fn process_file(path: &std::path::Path, dry_run: bool) -> Result<()> {
    let source_code = std::fs::read_to_string(path)?;
    
    // Parse the PHP file
    let interner = mago_interner::ThreadedInterner::new();
    let source_id = interner.intern(path.to_string_lossy().as_ref());
    let source = mago_source::Source::new(source_id, source_code.clone());
    
    let result = mago_syntax::parse(&source);
    
    if !result.errors.is_empty() {
        eprintln!("Parse errors in {:?}, skipping", path);
        return Ok(());
    }
    
    // Apply refactoring rules
    let edits = rules::array_push::check(&result.program, &source_code);
    
    if edits.is_empty() {
        return Ok(());
    }
    
    println!("{}:", path.display());
    
    // Apply edits
    let new_source = editor::apply_edits(&source_code, &edits);
    
    if dry_run {
        // Show diff
        print_diff(&source_code, &new_source);
    } else {
        std::fs::write(path, new_source)?;
        println!("  Applied {} changes", edits.len());
    }
    
    Ok(())
}

fn print_diff(old: &str, new: &str) {
    use colored::*;
    
    for diff in diff::lines(old, new) {
        match diff {
            diff::Result::Left(l) => println!("{}", format!("- {}", l).red()),
            diff::Result::Right(r) => println!("{}", format!("+ {}", r).green()),
            diff::Result::Both(_, _) => {}
        }
    }
}
```

### src/editor.rs

```rust
//! Span-based source code editor with format preservation

use mago_span::Span;

#[derive(Debug, Clone)]
pub struct Edit {
    pub span: Span,
    pub replacement: String,
    pub message: String,
}

impl Edit {
    pub fn new(span: Span, replacement: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            message: message.into(),
        }
    }
}

/// Apply edits to source code, preserving formatting where possible
pub fn apply_edits(source: &str, edits: &[Edit]) -> String {
    if edits.is_empty() {
        return source.to_string();
    }
    
    // Sort edits by position (reverse order for safe replacement)
    let mut sorted_edits: Vec<_> = edits.iter().collect();
    sorted_edits.sort_by(|a, b| b.span.start.offset.cmp(&a.span.start.offset));
    
    let mut result = source.to_string();
    
    for edit in sorted_edits {
        let start = edit.span.start.offset as usize;
        let end = edit.span.end.offset as usize;
        
        // Get the original text to preserve any leading/trailing whitespace patterns
        let original = &source[start..end];
        let replacement = adjust_whitespace(original, &edit.replacement);
        
        result.replace_range(start..end, &replacement);
    }
    
    result
}

/// Attempt to preserve whitespace patterns from original code
fn adjust_whitespace(original: &str, replacement: &str) -> String {
    // Simple heuristic: preserve leading whitespace
    let leading_ws: String = original
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    
    if !leading_ws.is_empty() && !replacement.starts_with(&leading_ws) {
        format!("{}{}", leading_ws, replacement.trim_start())
    } else {
        replacement.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mago_span::{Position, Span};
    
    fn make_span(start: u32, end: u32) -> Span {
        Span::new(
            Position::new(0.into(), start, 1, start + 1),
            Position::new(0.into(), end, 1, end + 1),
        )
    }
    
    #[test]
    fn test_simple_replacement() {
        let source = "array_push($arr, $val);";
        let edit = Edit::new(
            make_span(0, 22),
            "$arr[] = $val",
            "test",
        );
        
        let result = apply_edits(source, &[edit]);
        assert_eq!(result, "$arr[] = $val;");
    }
}
```

### src/rules/mod.rs

```rust
pub mod array_push;
```

### src/rules/array_push.rs

```rust
//! Rule: Convert array_push($arr, $val) to $arr[] = $val
//! 
//! This is more performant as it avoids function call overhead.

use mago_syntax::ast::*;
use mago_span::HasSpan;
use crate::editor::Edit;

pub fn check(program: &Program, source: &str) -> Vec<Edit> {
    let mut edits = Vec::new();
    
    visit_statements(&program.statements, source, &mut edits);
    
    edits
}

fn visit_statements(statements: &[Statement], source: &str, edits: &mut Vec<Edit>) {
    for stmt in statements {
        visit_statement(stmt, source, edits);
    }
}

fn visit_statement(stmt: &Statement, source: &str, edits: &mut Vec<Edit>) {
    match stmt {
        Statement::Expression(expr_stmt) => {
            check_expression(&expr_stmt.expression, source, edits);
        }
        Statement::Block(block) => {
            visit_statements(&block.statements, source, edits);
        }
        Statement::If(if_stmt) => {
            visit_statement(&if_stmt.then, source, edits);
            if let Some(else_clause) = &if_stmt.else_clause {
                visit_statement(&else_clause.statement, source, edits);
            }
        }
        Statement::Foreach(foreach) => {
            visit_statement(&foreach.body, source, edits);
        }
        Statement::For(for_stmt) => {
            visit_statement(&for_stmt.body, source, edits);
        }
        Statement::While(while_stmt) => {
            visit_statement(&while_stmt.body, source, edits);
        }
        Statement::Function(func) => {
            visit_statements(&func.body.statements, source, edits);
        }
        Statement::Class(class) => {
            for member in &class.members {
                if let ClassMember::Method(method) = member {
                    if let Some(body) = &method.body {
                        visit_statements(&body.statements, source, edits);
                    }
                }
            }
        }
        // Add more statement types as needed
        _ => {}
    }
}

fn check_expression(expr: &Expression, source: &str, edits: &mut Vec<Edit>) {
    // Check if this is array_push($arr, $val) call
    if let Expression::Call(call) = expr {
        if let Expression::Identifier(ident) = &*call.target {
            let name = &source[ident.span().start.offset as usize..ident.span().end.offset as usize];
            
            if name == "array_push" {
                // Check arguments
                if let Some(args) = &call.arguments {
                    let arg_list: Vec<_> = args.arguments.iter().collect();
                    
                    // Only handle simple case: array_push($arr, $single_value)
                    if arg_list.len() == 2 {
                        let arr_span = arg_list[0].span();
                        let val_span = arg_list[1].span();
                        
                        let arr_code = &source[arr_span.start.offset as usize..arr_span.end.offset as usize];
                        let val_code = &source[val_span.start.offset as usize..val_span.end.offset as usize];
                        
                        let replacement = format!("{}[] = {}", arr_code, val_code);
                        
                        edits.push(Edit::new(
                            call.span(),
                            replacement,
                            "Replace array_push() with short syntax for better performance",
                        ));
                    }
                }
            }
        }
    }
    
    // Recursively check nested expressions
    // (This is simplified - a proper implementation would use a visitor pattern)
}
```

## Step 4: Test It

Create a test PHP file:

```php
<?php
// test.php

$items = [];

// Should be transformed
array_push($items, 'foo');
array_push($items, $bar);
array_push($data['key'], getValue());

// Should NOT be transformed (multiple values)
array_push($items, 'a', 'b', 'c');

// Should NOT be transformed (return value used)
$count = array_push($items, 'foo');

class Example {
    public function test() {
        array_push($this->items, $value);
    }
}
```

Run the PoC:

```bash
cargo run -- --dry-run test.php
```

Expected output:
```diff
test.php:
- array_push($items, 'foo');
+ $items[] = 'foo';
- array_push($items, $bar);
+ $items[] = $bar;
...
```

## Step 5: Benchmarking

Create a benchmark script:

```bash
#!/bin/bash
# benchmark.sh

# Generate test files
mkdir -p /tmp/php-bench
for i in $(seq 1 1000); do
    cat > /tmp/php-bench/file_$i.php << 'EOF'
<?php
$arr = [];
array_push($arr, 'value1');
array_push($arr, $var);
array_push($arr, getValue());
for ($i = 0; $i < 10; $i++) {
    array_push($arr, $i);
}
EOF
done

echo "=== php-refactor-poc ==="
time cargo run --release -- --dry-run /tmp/php-bench > /dev/null

echo ""
echo "=== Rector (if installed) ==="
# Create rector config
cat > /tmp/rector.php << 'EOF'
<?php
use Rector\Config\RectorConfig;
use Rector\CodeQuality\Rector\FuncCall\ArrayPushShortSyntaxRector;

return RectorConfig::configure()
    ->withPaths(['/tmp/php-bench'])
    ->withRules([ArrayPushShortSyntaxRector::class]);
EOF

time vendor/bin/rector process --dry-run --config=/tmp/rector.php > /dev/null

# Cleanup
rm -rf /tmp/php-bench /tmp/rector.php
```

## Next Steps After PoC

Once you have the basic PoC working:

1. **Study mago-fixer** more deeply to understand their edit application strategy
2. **Add visitor pattern** for cleaner AST traversal
3. **Handle edge cases** in array_push (return value used, multiple values)
4. **Add 2-3 more simple rules** to validate the architecture
5. **Benchmark against Rector** on your real codebase

## Questions to Answer in Phase 0

- [ ] Does mago-syntax preserve enough span information for CST-style edits?
- [ ] How does mago-fixer handle overlapping edits?
- [ ] What's the performance profile? (parsing vs rule application)
- [ ] Can we use mago-walker for cleaner AST traversal?
- [ ] What type information is available from mago-semantics?
