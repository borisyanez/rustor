# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is a **documentation repository** containing design plans for PHPRefactor - a proposed Rust-based PHP code refactoring tool that aims to be a high-performance alternative to Rector. There is no implementation code yet; these are planning documents for a future project.

## Document Structure

- `php-refactor-rust-plan.md` - Main project plan covering all phases (0-5), architecture, rule system design, and implementation strategy
- `php-refactor-phase0-quickstart.md` - Quick start guide for Phase 0 with code examples for the proof-of-concept
- `php-refactor-class-move-test.md` - Comprehensive "acid test" case for class move refactoring with all reference types
- `../php-refactor-complete-plan.md` - Complete consolidated plan document

## Key Technical Context

**Foundation**: The project builds on [Mago](https://github.com/carthage-software/mago) crate ecosystem:
- `mago-syntax` - PHP parser and AST
- `mago-fixer` - Code fix application (critical for refactoring)
- `mago-span` - Source positions for format-preserving edits
- `mago-walker` - AST visitor pattern

**Core Architecture**:
- CST-based span editing for format preservation
- Parallel file processing with `rayon`
- Hybrid rule system: Rust traits for complex rules, TOML/YAML for simple patterns
- Type-aware refactoring via `mago-analyzer` and PHPDoc parsing

**Target Performance**: 10-50x faster than Rector (~5000+ files/sec vs ~100 files/sec)

## When Working on These Documents

- Maintain consistency with the planned Rust workspace structure in `php-refactor/crates/`
- Reference the Mago crate APIs as documented in the plans
- The "class move test" in `php-refactor-class-move-test.md` defines 28+ reference types that must be handled - use this as the validation checklist
- Phase timeline estimates should not be treated as commitments - focus on deliverables not dates
