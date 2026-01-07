# rustor-core

Core library for rustor PHP refactoring tool.

## Overview

This crate provides the fundamental types and utilities used by rustor:

- **Edit** - Represents a text replacement with position and message
- **apply_edits** - Applies multiple edits to source code
- **Visitor** - Trait for traversing PHP AST

## Usage

```rust
use rustor_core::{Edit, apply_edits};
use mago_span::Span;

// Create an edit
let edit = Edit::new(
    span,
    "replacement_text".to_string(),
    "Description of change",
);

// Apply edits to source
let new_source = apply_edits(source, &[edit])?;
```

## Types

### Edit

```rust
pub struct Edit {
    pub span: Span,          // Position in source
    pub replacement: String, // New text
    pub message: String,     // Human-readable description
}
```

### Visitor Trait

```rust
pub trait Visitor<'a> {
    fn visit_program(&mut self, program: &Program<'a>, source: &str);
    fn visit_statement(&mut self, stmt: &Statement<'a>, source: &str) -> bool;
    fn visit_expression(&mut self, expr: &Expression<'a>, source: &str) -> bool;
}
```

The visitor provides default implementations that traverse the entire AST. Override methods to handle specific node types.

## Design Principles

- **Minimal dependencies** - Only depends on mago-span
- **No PHP parsing** - Parsing is done by consumers
- **Format-preserving** - Edits work on byte offsets, preserving surrounding code

## See Also

- [rustor-rules](../rustor-rules/README.md) - Rule implementations
- [rustor-cli](../rustor-cli/README.md) - CLI application
