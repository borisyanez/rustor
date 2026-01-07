# PHPRefactor: A Rust-based Rector Alternative

## Complete Project Plan (Phases 0-5)

**Project Codename**: `php-refactor` (alternatives: `refactor-php`, `phpmod`, `transmute`)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Foundation: Mago Crate Ecosystem](#foundation-mago-crate-ecosystem)
3. [Phase 0: Research & Prototyping](#phase-0-research--prototyping-2-3-weeks)
4. [Phase 1: Core Infrastructure](#phase-1-core-infrastructure-4-6-weeks)
5. [Phase 2: Hybrid Rule Definition](#phase-2-hybrid-rule-definition-3-4-weeks)
6. [Phase 3: Type-Aware Refactoring](#phase-3-type-aware-refactoring-4-6-weeks)
7. [Phase 4: Advanced Features](#phase-4-advanced-features-4-6-weeks)
8. [Phase 5: Framework Support & Ecosystem](#phase-5-framework-support--ecosystem-ongoing)
9. [Timeline Summary](#timeline-summary)
10. [Technical Risks & Mitigations](#technical-risks--mitigations)
11. [Success Metrics](#success-metrics)
12. [Appendix A: Phase 0 Quick Start Guide](#appendix-a-phase-0-quick-start-guide)
13. [Appendix B: Class Move Test Case (Acid Test)](#appendix-b-class-move-test-case-acid-test)

---

## Executive Summary

Build a high-performance PHP code refactoring tool in Rust that can perform large-scale codebase transformations 10-50x faster than Rector, with format-preserving output, type-aware transformations, and a hybrid rule definition system.

### Key Differentiators from Rector

| Aspect | Rector | PHPRefactor |
|--------|--------|-------------|
| Language | PHP | Rust |
| Parallelism | Single-threaded | Multi-core (rayon) |
| Memory | High (PHP GC) | Low (zero-copy where possible) |
| Speed | ~100 files/sec | ~5000+ files/sec target |
| Format preservation | Via php-parser printer | CST-based span editing |
| Type info | PHPStan integration | Native type inference |

### Why Build This?

1. **Performance**: Rector on large codebases (50k+ files) can take 30+ minutes. A Rust implementation targeting 10x speedup makes iterative refactoring practical.

2. **Format Preservation**: Rector's printer often reformats code, creating noisy diffs. CST-based span editing preserves original formatting.

3. **Parallelism**: PHP's execution model limits Rector to single-threaded operation. Rust + rayon enables true parallel processing.

4. **Type Safety**: Build type inference directly into the tool rather than depending on external tools like PHPStan.

---

## Foundation: Mago Crate Ecosystem

Based on research, Mago provides these reusable crates:

```
mago-syntax        → Lexer, Parser, AST structures
mago-ast           → AST node definitions  
mago-ast-utils     → AST traversal helpers
mago-span          → Source positions/spans
mago-source        → Source file management
mago-interner      → String interning for performance
mago-names         → Name resolution
mago-symbol-table  → Symbol table construction
mago-semantics     → Semantic analysis pipeline
mago-analyzer      → Type inference & analysis
mago-fixer         → Code fix application (!)
mago-walker        → AST visitor pattern
mago-reporting     → Diagnostic reporting
mago-php-version   → PHP version feature detection
mago-reference     → Cross-file symbol lookup
```

**Critical insight**: `mago-fixer` already exists for applying code fixes — this is the foundation for refactoring.

---

## Phase 0: Research & Prototyping (2-3 weeks)

### Goals
- Deep-dive into Mago internals
- Validate CST preservation approach
- Build minimal proof-of-concept

### Tasks

#### 0.1 Mago Exploration
- [ ] Clone and build mago locally
- [ ] Study `mago-fixer` implementation
- [ ] Understand how lint auto-fixes work
- [ ] Map out the fix application pipeline
- [ ] Document span handling and source map approach

#### 0.2 CST/Format Preservation Research
- [ ] Analyze how Mago's formatter handles whitespace/comments
- [ ] Study tree-sitter's approach (for comparison)
- [ ] Design span-based edit strategy
- [ ] Prototype: Parse → trivial transform → reprint with minimal diff

#### 0.3 Proof of Concept
- [ ] Create standalone Rust project depending on mago crates
- [ ] Implement single rule: `array_push($a, $b)` → `$a[] = $b`
- [ ] Verify output preserves surrounding formatting
- [ ] Benchmark against Rector on same transformation

### Deliverables
- Technical design document
- Working PoC with 1 rule
- Performance benchmark comparison

### Questions to Answer
- Does mago-syntax preserve enough span information for CST-style edits?
- How does mago-fixer handle overlapping edits?
- What's the performance profile? (parsing vs rule application)
- Can we use mago-walker for cleaner AST traversal?
- What type information is available from mago-semantics?

---

## Phase 1: Core Infrastructure (4-6 weeks)

### Goals
- Establish project structure
- Build rule engine foundation
- Implement CST-preserving code writer

### 1.1 Project Structure

```
php-refactor/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── php-refactor-core/     # Core abstractions
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── rule.rs        # Rule trait definitions
│   │   │   ├── context.rs     # Refactoring context
│   │   │   ├── edit.rs        # Edit/change representation
│   │   │   └── config.rs      # Configuration structures
│   │   └── Cargo.toml
│   │
│   ├── php-refactor-edits/    # CST-preserving editor
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── span_edit.rs   # Span-based editing
│   │   │   ├── trivia.rs      # Whitespace/comment handling
│   │   │   └── printer.rs     # Minimal-diff code printer
│   │   └── Cargo.toml
│   │
│   ├── php-refactor-rules/    # Built-in rule implementations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── php/           # PHP version upgrade rules
│   │   │   ├── quality/       # Code quality rules
│   │   │   └── patterns/      # Pattern-based simple rules
│   │   └── Cargo.toml
│   │
│   ├── php-refactor-config/   # TOML/YAML rule parsing
│   │   └── ...
│   │
│   └── php-refactor-cli/      # CLI application
│       └── ...
│
├── rules/                     # User-defined YAML rules
│   └── examples/
│
└── tests/
    ├── fixtures/              # Test PHP files
    └── integration/           # Integration tests
```

### 1.2 Core Rule Trait

```rust
// crates/php-refactor-core/src/rule.rs

use mago_ast::Program;
use mago_span::Span;

/// Represents a single code edit
#[derive(Debug, Clone)]
pub struct Edit {
    pub span: Span,           // What to replace
    pub replacement: String,   // New code
    pub message: String,       // Description for user
}

/// Context provided to rules during analysis
pub struct RefactorContext<'a> {
    pub source: &'a mago_source::Source,
    pub program: &'a Program,
    pub symbols: &'a mago_symbol_table::SymbolTable,
    pub types: Option<&'a TypeInfo>,  // Phase 3
    pub php_version: PhpVersion,
    pub config: &'a RuleConfig,
}

/// The core rule trait - implemented in Rust
pub trait Rule: Send + Sync {
    /// Unique identifier for the rule
    fn id(&self) -> &'static str;
    
    /// Human-readable name
    fn name(&self) -> &'static str;
    
    /// Description of what the rule does
    fn description(&self) -> &'static str;
    
    /// Which PHP versions this rule applies to
    fn applicable_versions(&self) -> VersionRange {
        VersionRange::all()
    }
    
    /// Analyze the AST and return edits
    fn check(&self, ctx: &RefactorContext) -> Vec<Edit>;
    
    /// Optional: Check if rule is enabled for this context
    fn is_enabled(&self, ctx: &RefactorContext) -> bool {
        true
    }
}

/// For rules that need to visit specific node types
pub trait NodeRule<N>: Rule {
    fn check_node(&self, node: &N, ctx: &RefactorContext) -> Vec<Edit>;
}
```

### 1.3 CST-Preserving Editor

```rust
// crates/php-refactor-edits/src/span_edit.rs

use mago_span::Span;

/// Applies edits to source while preserving formatting
pub struct SourceEditor<'a> {
    original: &'a str,
    edits: Vec<Edit>,
}

impl<'a> SourceEditor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            original: source,
            edits: Vec::new(),
        }
    }
    
    pub fn add_edit(&mut self, edit: Edit) {
        self.edits.push(edit);
    }
    
    /// Apply all edits and return new source
    /// Handles overlapping spans, maintains relative positions
    pub fn apply(mut self) -> String {
        // Sort edits by span start (reverse order for safe replacement)
        self.edits.sort_by(|a, b| b.span.start.cmp(&a.span.start));
        
        let mut result = self.original.to_string();
        
        for edit in self.edits {
            let start = edit.span.start.offset as usize;
            let end = edit.span.end.offset as usize;
            
            // Preserve leading/trailing trivia from original span
            let replacement = self.preserve_trivia(
                &self.original[start..end],
                &edit.replacement
            );
            
            result.replace_range(start..end, &replacement);
        }
        
        result
    }
    
    fn preserve_trivia(&self, original: &str, replacement: &str) -> String {
        // Extract leading whitespace/comments from original
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
}
```

### 1.4 Parallel Execution Engine

```rust
// crates/php-refactor-core/src/engine.rs

use rayon::prelude::*;

pub struct RefactorEngine {
    rules: Vec<Box<dyn Rule>>,
    config: Config,
}

impl RefactorEngine {
    pub fn run(&self, files: Vec<PathBuf>) -> RefactorResult {
        // Phase 1: Parse all files in parallel
        let parsed: Vec<_> = files
            .par_iter()
            .map(|path| self.parse_file(path))
            .collect();
        
        // Phase 2: Build cross-file symbol table (if needed)
        let symbols = self.build_symbol_table(&parsed);
        
        // Phase 3: Apply rules in parallel
        let edits: Vec<_> = parsed
            .par_iter()
            .flat_map(|file| {
                let ctx = RefactorContext::new(file, &symbols, &self.config);
                self.rules
                    .iter()
                    .flat_map(|rule| rule.check(&ctx))
                    .collect::<Vec<_>>()
            })
            .collect();
        
        RefactorResult { edits }
    }
}
```

### 1.5 CLI Application

```rust
// crates/php-refactor-cli/src/main.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "php-refactor")]
#[command(about = "High-performance PHP refactoring tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run refactoring rules
    Run {
        /// Paths to process
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        
        /// Show changes without applying
        #[arg(long, short = 'n')]
        dry_run: bool,
        
        /// Configuration file
        #[arg(long, short = 'c')]
        config: Option<PathBuf>,
        
        /// Output format (text, json, diff)
        #[arg(long, default_value = "text")]
        format: String,
    },
    
    /// Initialize configuration
    Init {
        /// PHP version target
        #[arg(long, default_value = "8.2")]
        php_version: String,
    },
    
    /// List available rules
    List {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
    },
    
    /// Move a class to a new namespace
    MoveClass {
        /// Source file
        #[arg(long)]
        from: PathBuf,
        
        /// Target directory
        #[arg(long)]
        to: PathBuf,
        
        /// New namespace
        #[arg(long)]
        namespace: String,
        
        /// Dry run
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
}
```

### Deliverables Phase 1
- Working CLI that can apply Rust-defined rules
- 5-10 simple rules (array_push, count in loop, etc.)
- Parallel file processing
- Basic format preservation
- `--dry-run` mode with diff output

---

## Phase 2: Hybrid Rule Definition (3-4 weeks)

### Goals
- TOML/YAML rule definition for simple patterns
- Rule composition and sets
- User-extensible configuration

### 2.1 Declarative Rule Format (TOML)

```toml
# rules/array-push.toml
[rule]
id = "array-push-to-short-syntax"
name = "Replace array_push with short syntax"
description = "Converts array_push($a, $b) to $a[] = $b"
category = "performance"
fixable = true

[rule.php_versions]
min = "5.4"

[match]
type = "function_call"
name = "array_push"
args = { count = 2 }

[transform]
pattern = """
{args[0]}[] = {args[1]}
"""

# Optional: conditions
[conditions]
# Only when array_push returns value is not used
return_value_used = false
```

### 2.2 Declarative Rule Format (YAML)

```yaml
# rules/constructor-promotion.yaml
rule:
  id: constructor-property-promotion
  name: Use constructor property promotion
  php_versions:
    min: "8.0"
  
match:
  type: class
  has:
    - type: property
      capture: $prop
    - type: constructor
      has:
        - type: parameter
          name: "{$prop.name}"
        - type: assignment
          left: "this->{$prop.name}"
          right: "{$prop.name}"

transform:
  # Remove property declaration
  remove: $prop
  # Modify constructor parameter  
  modify:
    target: constructor.parameter
    add_visibility: "{$prop.visibility}"
  # Remove assignment in constructor
  remove: constructor.assignment
```

### 2.3 Rule Configuration Schema

```rust
// crates/php-refactor-config/src/rule_def.rs

#[derive(Debug, Deserialize)]
pub struct RuleDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub php_versions: Option<VersionConstraint>,
    pub fixable: bool,
    
    #[serde(rename = "match")]
    pub matcher: MatcherDef,
    
    pub transform: TransformDef,
    
    pub conditions: Option<Vec<Condition>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum MatcherDef {
    FunctionCall {
        name: StringPattern,
        args: Option<ArgsConstraint>,
    },
    MethodCall {
        object: Option<TypePattern>,
        method: StringPattern,
    },
    Class {
        has: Vec<MatcherDef>,
        #[serde(default)]
        capture: Option<String>,
    },
    Property { /* ... */ },
    BinaryExpression {
        operator: String,
        left: Box<MatcherDef>,
        right: Box<MatcherDef>,
    },
    Literal {
        value: serde_json::Value,
    },
    // ... more node types
}

#[derive(Debug, Deserialize)]
pub struct TransformDef {
    pub pattern: Option<String>,      // Template string
    pub remove: Option<Vec<String>>,  // Captures to remove
    pub modify: Option<Vec<ModifyDef>>,
}

#[derive(Debug, Deserialize)]
pub struct ModifyDef {
    pub target: String,
    pub add_visibility: Option<String>,
    pub rename: Option<String>,
    pub wrap: Option<String>,
}
```

### 2.4 Rule Sets (Like Rector's SetLists)

```toml
# sets/php82.toml
[set]
id = "php82"
name = "PHP 8.2 Upgrade"
description = "Rules for upgrading to PHP 8.2"

[[rules]]
ref = "readonly-classes"

[[rules]]
ref = "null-false-standalone-types"

[[rules]]
ref = "constants-in-traits"

[[rules]]
# Inline simple rule
id = "deprecated-utf8-encode"
match = { type = "function_call", name = "utf8_encode" }
transform = { pattern = "mb_convert_encoding({args[0]}, 'UTF-8', 'ISO-8859-1')" }
message = "utf8_encode is deprecated in PHP 8.2"
```

### 2.5 Project Configuration

```toml
# php-refactor.toml (project config)
[project]
php_version = "8.2"
paths = ["src", "app"]
exclude = ["vendor", "tests/fixtures"]

[rules]
# Enable rule sets
sets = ["php82", "code-quality", "dead-code"]

# Enable individual rules
enable = [
    "array-push-to-short-syntax",
    "constructor-property-promotion",
]

# Disable specific rules from sets
disable = [
    "some-annoying-rule",
]

# Rule-specific configuration
[rules.config.max-line-length]
length = 120

[rules.config.naming-convention]
class_pattern = "^[A-Z][a-zA-Z0-9]*$"
method_pattern = "^[a-z][a-zA-Z0-9]*$"
```

### 2.6 Built-in Rule Sets

| Set ID | Description | Rule Count |
|--------|-------------|------------|
| `php74` | PHP 7.4 features | ~15 rules |
| `php80` | PHP 8.0 upgrade | ~25 rules |
| `php81` | PHP 8.1 upgrade | ~20 rules |
| `php82` | PHP 8.2 upgrade | ~15 rules |
| `php83` | PHP 8.3 upgrade | ~10 rules |
| `php84` | PHP 8.4 upgrade | ~10 rules |
| `code-quality` | General improvements | ~30 rules |
| `dead-code` | Remove unused code | ~15 rules |
| `type-declarations` | Add type hints | ~20 rules |
| `early-return` | Reduce nesting | ~10 rules |

### Deliverables Phase 2
- TOML/YAML rule parser
- 20+ declarative rules for PHP upgrades
- Rule sets for PHP 7.4→8.0, 8.0→8.1, 8.1→8.2, 8.2→8.3, 8.3→8.4
- Project configuration file support
- `php-refactor init` command
- `php-refactor list` command with filtering

---

## Phase 3: Type-Aware Refactoring (4-6 weeks)

### Goals
- Integrate type inference from mago-analyzer
- PHPDoc parsing for legacy type info
- Type-dependent rule transformations

### 3.1 Type System Integration

```rust
// crates/php-refactor-types/src/lib.rs

use std::collections::HashMap;

pub struct TypeInfo {
    /// Maps expressions to their inferred types
    expression_types: HashMap<NodeId, Type>,
    
    /// Maps variables to types at specific program points
    variable_types: HashMap<(ScopeId, VariableId), Type>,
    
    /// Class hierarchy information
    class_hierarchy: ClassHierarchy,
}

impl TypeInfo {
    pub fn type_of(&self, expr: &Expression) -> Option<&Type> {
        self.expression_types.get(&expr.id())
    }
    
    pub fn variable_type(&self, scope: ScopeId, var: &Variable) -> Option<&Type> {
        self.variable_types.get(&(scope, var.id()))
    }
    
    pub fn is_subtype_of(&self, sub: &Type, super_: &Type) -> bool {
        match (sub, super_) {
            (Type::Class(sub_name), Type::Class(super_name)) => {
                self.class_hierarchy.is_subclass_of(sub_name, super_name)
            }
            (Type::Nullable(inner), other) => {
                self.is_subtype_of(inner, other)
            }
            // ... more cases
            _ => sub == super_
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Scalar types
    Int,
    Float,
    String,
    Bool,
    Null,
    
    // Compound types
    Array(Option<Box<Type>>),          // array or array<T>
    Class(String),                      // Fully qualified class name
    Interface(String),
    
    // Type combinators
    Nullable(Box<Type>),               // ?T
    Union(Vec<Type>),                  // T|U
    Intersection(Vec<Type>),           // T&U
    
    // Special types
    Mixed,
    Void,
    Never,
    Object,
    Callable,
    Iterable,
    
    // Generic types (from PHPDoc)
    Generic(String, Vec<Type>),        // Collection<T, U>
    
    // Unknown/inference failed
    Unknown,
}
```

### 3.2 Type-Aware Rule Example

```rust
// rules/type-aware/instanceof-vs-getclass.rs

/// Converts `get_class($x) === Foo::class` to `$x instanceof Foo`
/// Only when $x is guaranteed to be an object
pub struct GetClassToInstanceof;

impl Rule for GetClassToInstanceof {
    fn id(&self) -> &'static str { "getclass-to-instanceof" }
    fn name(&self) -> &'static str { "Replace get_class() comparison with instanceof" }
    fn description(&self) -> &'static str {
        "Converts get_class($x) === Foo::class to $x instanceof Foo for better readability"
    }
    
    fn check(&self, ctx: &RefactorContext) -> Vec<Edit> {
        let mut edits = Vec::new();
        
        // Find binary expressions: get_class($x) === Foo::class
        for expr in ctx.program.find_all::<BinaryExpression>() {
            if !matches!(expr.operator, BinaryOp::Identical | BinaryOp::Equal) {
                continue;
            }
            
            // Check if left side is get_class() call
            let Some(call) = expr.left.as_function_call() else { continue };
            if call.name.as_str() != "get_class" { continue }
            
            // Check if right side is Class::class
            let Some(class_const) = expr.right.as_class_constant() else { continue };
            if class_const.constant.as_str() != "class" { continue }
            
            // TYPE-AWARE CHECK: Ensure argument is object type
            let arg = &call.arguments[0];
            if let Some(types) = ctx.types {
                let arg_type = types.type_of(arg);
                if !arg_type.map(|t| t.is_object()).unwrap_or(false) {
                    // Skip if we can't confirm it's an object
                    // get_class() on non-object throws in PHP 8+
                    continue;
                }
            }
            
            // Generate replacement
            let var_code = ctx.source.slice(arg.span());
            let class_name = class_const.class.as_str();
            
            edits.push(Edit {
                span: expr.span(),
                replacement: format!("{} instanceof {}", var_code, class_name),
                message: "Use instanceof instead of get_class() comparison".into(),
            });
        }
        
        edits
    }
}
```

### 3.3 PHPDoc Type Extraction

```rust
// crates/php-refactor-types/src/phpdoc.rs

use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref VAR_PATTERN: Regex = Regex::new(r"@var\s+(\S+)").unwrap();
    static ref PARAM_PATTERN: Regex = Regex::new(r"@param\s+(\S+)\s+\$(\w+)").unwrap();
    static ref RETURN_PATTERN: Regex = Regex::new(r"@return\s+(\S+)").unwrap();
    static ref GENERIC_PATTERN: Regex = Regex::new(r"(\w+)<(.+)>").unwrap();
}

/// Parses PHPDoc blocks for type information
pub struct PhpDocParser;

impl PhpDocParser {
    pub fn parse_var_type(&self, doc: &str) -> Option<Type> {
        VAR_PATTERN.captures(doc)
            .and_then(|c| c.get(1))
            .map(|m| self.parse_type_string(m.as_str()))
    }
    
    pub fn parse_param_types(&self, doc: &str) -> HashMap<String, Type> {
        PARAM_PATTERN.captures_iter(doc)
            .filter_map(|c| {
                let type_str = c.get(1)?.as_str();
                let name = c.get(2)?.as_str();
                Some((name.to_string(), self.parse_type_string(type_str)))
            })
            .collect()
    }
    
    pub fn parse_return_type(&self, doc: &str) -> Option<Type> {
        RETURN_PATTERN.captures(doc)
            .and_then(|c| c.get(1))
            .map(|m| self.parse_type_string(m.as_str()))
    }
    
    fn parse_type_string(&self, s: &str) -> Type {
        // Handle nullable
        if s.starts_with('?') {
            return Type::Nullable(Box::new(self.parse_type_string(&s[1..])));
        }
        
        // Handle union types
        if s.contains('|') {
            let types: Vec<Type> = s.split('|')
                .map(|t| self.parse_type_string(t.trim()))
                .collect();
            return Type::Union(types);
        }
        
        // Handle generics: Collection<T, U>
        if let Some(caps) = GENERIC_PATTERN.captures(s) {
            let base = caps.get(1).unwrap().as_str();
            let params: Vec<Type> = caps.get(2).unwrap().as_str()
                .split(',')
                .map(|t| self.parse_type_string(t.trim()))
                .collect();
            return Type::Generic(base.to_string(), params);
        }
        
        // Handle simple types
        match s.to_lowercase().as_str() {
            "int" | "integer" => Type::Int,
            "float" | "double" => Type::Float,
            "string" => Type::String,
            "bool" | "boolean" => Type::Bool,
            "null" => Type::Null,
            "array" => Type::Array(None),
            "object" => Type::Object,
            "mixed" => Type::Mixed,
            "void" => Type::Void,
            "never" => Type::Never,
            "callable" => Type::Callable,
            "iterable" => Type::Iterable,
            _ => Type::Class(s.to_string()),
        }
    }
}
```

### 3.4 Type-Dependent Declarative Rules

```yaml
# rules/type-aware/string-functions.yaml
rule:
  id: str-contains-polyfill
  name: Replace str_contains polyfill
  php_versions:
    min: "8.0"

match:
  type: binary_expression
  operator: "!=="
  left:
    type: function_call
    name: strpos
    args:
      - capture: $haystack
      - capture: $needle
  right:
    type: literal
    value: false

# Type conditions - only apply if types are confirmed
conditions:
  - capture: $haystack
    type: string
  - capture: $needle  
    type: string

transform:
  pattern: "str_contains({$haystack}, {$needle})"
```

```yaml
# rules/type-aware/array-key-exists.yaml
rule:
  id: isset-to-array-key-exists
  name: Replace isset() with array_key_exists() for null values
  
match:
  type: function_call
  name: isset
  args:
    - type: array_access
      array:
        capture: $array
      key:
        capture: $key

# Only when array might contain null values
conditions:
  - capture: $array
    type: "array<mixed>"  # Array that might contain nulls

transform:
  pattern: "array_key_exists({$key}, {$array})"
```

### 3.5 Add Return Types Based on Inference

```rust
// rules/type-aware/add-return-type.rs

pub struct AddReturnTypeDeclaration;

impl Rule for AddReturnTypeDeclaration {
    fn id(&self) -> &'static str { "add-return-type" }
    
    fn check(&self, ctx: &RefactorContext) -> Vec<Edit> {
        let mut edits = Vec::new();
        
        for method in ctx.program.find_all::<Method>() {
            // Skip if already has return type
            if method.return_type.is_some() {
                continue;
            }
            
            // Skip constructors, destructors
            if method.name.as_str().starts_with("__") {
                continue;
            }
            
            // Try to infer return type
            let inferred = ctx.types
                .and_then(|t| t.infer_return_type(method));
            
            // Also check PHPDoc
            let phpdoc_type = method.doc_comment
                .as_ref()
                .and_then(|doc| PhpDocParser.parse_return_type(doc));
            
            // Use inferred type, fall back to PHPDoc
            let return_type = inferred.or(phpdoc_type);
            
            if let Some(typ) = return_type {
                // Don't add 'mixed' - it's not useful
                if typ == Type::Mixed || typ == Type::Unknown {
                    continue;
                }
                
                let type_str = typ.to_php_string();
                
                // Insert after closing parenthesis
                let insert_pos = method.parameters_span.end;
                
                edits.push(Edit {
                    span: Span::point(insert_pos),
                    replacement: format!(": {}", type_str),
                    message: format!("Add return type: {}", type_str),
                });
            }
        }
        
        edits
    }
}
```

### Deliverables Phase 3
- Type inference integration with mago-analyzer
- PHPDoc parser supporting complex types (generics, unions)
- 15+ type-aware rules
- Type info available in declarative rule conditions
- Add return type declarations based on inference
- Add property type declarations based on inference

---

## Phase 4: Advanced Features (4-6 weeks)

### Goals
- Cross-file refactoring
- Rename/move with reference updates
- Custom Rust rule plugin system

### 4.1 Cross-File Operations

**KEY MILESTONE: Class Move Refactoring**

This is the "acid test" for the refactoring tool — moving a class to a different namespace while updating ALL references across the codebase. See [Appendix B](#appendix-b-class-move-test-case-acid-test) for the complete test case.

#### Reference Types to Handle

| Category | Types |
|----------|-------|
| **Use Statements** | Simple, aliased (`as`), grouped (`{A, B}`) |
| **Type Hints** | Parameters, returns, properties, nullable, union, intersection |
| **Instantiation** | `new Class()`, `Class::create()` |
| **Static Access** | `Class::method()`, `Class::$prop`, `Class::CONST`, `Class::class` |
| **Inheritance** | `extends`, `implements`, `use` (traits) |
| **Control Flow** | `instanceof`, `catch (Class $e)` |
| **PHPDoc** | `@param`, `@return`, `@var`, `@throws`, `@see`, generics |
| **Strings** | `'App\\Old\\Class'` in configs |
| **Files** | `require_once 'path/Class.php'` |
| **Edge Cases** | Implicit same-namespace resolution, attributes |

#### Implementation

```rust
/// Comprehensive reference kinds for class move refactoring
pub enum ReferenceKind {
    // Use statements
    UseStatement { aliased: bool, alias: Option<String>, grouped: bool },
    
    // Type system
    TypeHint { nullable: bool, union: bool, intersection: bool },
    ReturnType { nullable: bool },
    PropertyType { nullable: bool },
    ParameterType { nullable: bool },
    
    // Instantiation & access
    Instantiation,           // new Class()
    StaticMethodCall,        // Class::method()
    StaticPropertyAccess,    // Class::$prop
    ConstantAccess,          // Class::CONST
    ClassConstant,           // Class::class
    
    // OOP relationships
    Extends,
    Implements,
    TraitUse,
    
    // Control flow
    InstanceOf,
    CatchClause,
    
    // Documentation
    PhpDocParam,
    PhpDocReturn,
    PhpDocVar,
    PhpDocThrows,
    PhpDocSee,
    PhpDocGeneric,           // Collection<Class>
    
    // Special cases
    StringLiteral,           // 'App\\Class' in configs
    Attribute,               // #[Class(param)]
    ImplicitNamespace,       // Same namespace, no use statement
    FileInclude,             // require/include path
}

/// Represents a reference to a class in the codebase
pub struct Reference {
    pub file: PathBuf,
    pub span: Span,
    pub kind: ReferenceKind,
    pub text: String,
    pub context: ReferenceContext,
}

/// Additional context for smart refactoring decisions
pub struct ReferenceContext {
    pub file_namespace: Option<String>,
    pub existing_uses: Vec<UseStatement>,
    pub is_same_namespace: bool,
}

/// Move a class to a new namespace
pub struct MoveClass {
    pub source_file: PathBuf,
    pub target_directory: PathBuf,
    pub new_namespace: String,
}

impl MoveClass {
    pub fn execute(&self, project: &Project) -> RefactorResult {
        let mut edits = Vec::new();
        
        // 1. Extract class info from source file
        let class_info = self.parse_class_file(&self.source_file)?;
        let old_fqn = class_info.fully_qualified_name();
        let new_fqn = format!("{}\\{}", self.new_namespace, class_info.name);
        
        // 2. Find ALL references using mago-reference
        let references = project.find_all_references(&old_fqn);
        
        // 3. Group by file for efficient processing
        let refs_by_file = references.group_by_file();
        
        // 4. Generate edits per file
        for (file, refs) in refs_by_file {
            let file_edits = self.process_file(&file, &refs, &old_fqn, &new_fqn)?;
            edits.extend(file_edits);
        }
        
        // 5. Update the class file itself
        edits.push(self.update_namespace_declaration(&class_info)?);
        
        // 6. Create file move operation
        let new_path = self.target_directory
            .join(format!("{}.php", class_info.name));
        edits.push(FileEdit::Move {
            from: self.source_file.clone(),
            to: new_path,
        });
        
        Ok(RefactorResult { edits })
    }
    
    fn process_file(
        &self,
        file: &Path,
        refs: &[Reference],
        old_fqn: &str,
        new_fqn: &str,
    ) -> Result<Vec<Edit>> {
        let mut edits = Vec::new();
        let ast = parse_file(file)?;
        
        // Determine namespace relationship
        let file_ns = ast.namespace();
        let old_ns = self.extract_namespace(old_fqn);
        let was_same_ns = file_ns.as_deref() == Some(old_ns);
        let is_same_ns = file_ns.as_deref() == Some(&self.new_namespace);
        
        // Handle use statement changes
        let has_use = ast.has_use_statement_for(old_fqn);
        let has_implicit = refs.iter().any(|r| 
            matches!(r.kind, ReferenceKind::ImplicitNamespace)
        );
        
        match (has_use, has_implicit, is_same_ns) {
            // Has use statement → update it
            (true, _, _) => {
                edits.push(self.update_use_statement(&ast, old_fqn, new_fqn)?);
            }
            // No use, was implicit, but now different namespace → add use
            (false, true, false) => {
                edits.push(self.add_use_statement(&ast, new_fqn)?);
            }
            // No use, now same namespace → implicit still works
            (false, true, true) => {}
            // No use, no implicit → only FQN references exist
            (false, false, _) => {}
        }
        
        // Update each reference by kind
        for ref_ in refs {
            match &ref_.kind {
                ReferenceKind::UseStatement { grouped: true, .. } => {
                    edits.push(self.handle_grouped_use(&ast, ref_, new_fqn)?);
                }
                ReferenceKind::StringLiteral => {
                    let escaped = new_fqn.replace("\\", "\\\\");
                    edits.push(Edit::replace(ref_.span, &escaped));
                }
                ReferenceKind::FileInclude => {
                    edits.push(self.update_include_path(ref_)?);
                }
                kind if kind.is_phpdoc() => {
                    edits.push(self.update_phpdoc(ref_, old_fqn, new_fqn)?);
                }
                kind if kind.is_fqn_reference() => {
                    edits.push(Edit::replace(ref_.span, new_fqn));
                }
                // Short names resolved via use statement - no change needed
                _ => {}
            }
        }
        
        Ok(edits)
    }
}
```

### 4.2 Plugin System (Dynamic Loading)

```rust
// crates/php-refactor-plugin/src/lib.rs

use libloading::{Library, Symbol};
use std::path::Path;

/// Trait for dynamically loaded rule plugins
pub trait RulePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn rules(&self) -> Vec<Box<dyn Rule>>;
}

/// Plugin manager for loading external rule sets
pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
}

struct LoadedPlugin {
    _library: Library,  // Keep library alive
    plugin: Box<dyn RulePlugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }
    
    /// Load plugin from shared library
    pub fn load(&mut self, path: &Path) -> Result<()> {
        unsafe {
            let lib = Library::new(path)?;
            
            let create_fn: Symbol<fn() -> Box<dyn RulePlugin>> = 
                lib.get(b"create_plugin")?;
            
            let plugin = create_fn();
            
            println!("Loaded plugin: {} v{}", plugin.name(), plugin.version());
            
            self.plugins.push(LoadedPlugin {
                _library: lib,
                plugin,
            });
        }
        
        Ok(())
    }
    
    pub fn all_rules(&self) -> Vec<&dyn Rule> {
        self.plugins
            .iter()
            .flat_map(|p| p.plugin.rules())
            .map(|r| r.as_ref())
            .collect()
    }
}

// Example plugin implementation (in separate crate)
// crates/my-custom-rules/src/lib.rs

pub struct MyCustomPlugin;

impl RulePlugin for MyCustomPlugin {
    fn name(&self) -> &str { "my-custom-rules" }
    fn version(&self) -> &str { "1.0.0" }
    
    fn rules(&self) -> Vec<Box<dyn Rule>> {
        vec![
            Box::new(MyCustomRule1),
            Box::new(MyCustomRule2),
        ]
    }
}

#[no_mangle]
pub fn create_plugin() -> Box<dyn RulePlugin> {
    Box::new(MyCustomPlugin)
}
```

### 4.3 Interactive Mode

```rust
// crates/php-refactor-cli/src/interactive.rs

use crossterm::{event, terminal};
use std::io::{self, Write};

pub async fn interactive_refactor(engine: &RefactorEngine, files: Vec<PathBuf>) -> Result<()> {
    let result = engine.run_dry(&files)?;
    
    if result.edits.is_empty() {
        println!("No changes to apply.");
        return Ok(());
    }
    
    println!("Found {} changes across {} files", 
             result.edits.len(), 
             result.affected_files().len());
    println!();
    
    let mut applied = 0;
    let mut skipped = 0;
    
    for (file, edits) in result.edits_by_file() {
        println!("━━━ {} ━━━", file.display());
        
        for edit in edits {
            // Show diff preview
            println!("{}", edit.diff_preview());
            println!();
            
            // Prompt for action
            print!("Apply this change? [y]es / [n]o / [q]uit / [a]ll remaining: ");
            io::stdout().flush()?;
            
            match read_char()? {
                'y' | 'Y' => {
                    edit.apply()?;
                    applied += 1;
                    println!("✓ Applied");
                }
                'n' | 'N' => {
                    skipped += 1;
                    println!("✗ Skipped");
                }
                'q' | 'Q' => {
                    println!("\nAborted. Applied {} changes, skipped {}.", applied, skipped);
                    return Ok(());
                }
                'a' | 'A' => {
                    // Apply all remaining
                    let remaining = result.remaining_edits();
                    for e in remaining {
                        e.apply()?;
                        applied += 1;
                    }
                    println!("\n✓ Applied all remaining changes.");
                    break;
                }
                _ => {
                    println!("Invalid input. Please enter y, n, q, or a.");
                }
            }
            println!();
        }
    }
    
    println!("\nComplete. Applied {} changes, skipped {}.", applied, skipped);
    
    Ok(())
}

fn read_char() -> Result<char> {
    terminal::enable_raw_mode()?;
    let result = loop {
        if let event::Event::Key(key) = event::read()? {
            if let event::KeyCode::Char(c) = key.code {
                break Ok(c);
            }
        }
    };
    terminal::disable_raw_mode()?;
    result
}
```

### 4.4 Watch Mode

```rust
// crates/php-refactor-cli/src/watch.rs

use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn watch_mode(engine: &RefactorEngine, paths: &[PathBuf], config: &Config) -> Result<()> {
    println!("Watching for changes... (Ctrl+C to stop)");
    
    let (tx, rx) = channel();
    
    let mut watcher = watcher(tx, Duration::from_millis(500))?;
    
    for path in paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }
    
    loop {
        match rx.recv() {
            Ok(event) => {
                if let Some(path) = event.path {
                    if path.extension().map_or(false, |e| e == "php") {
                        println!("\n📝 Changed: {}", path.display());
                        
                        match engine.run_single(&path) {
                            Ok(result) if !result.edits.is_empty() => {
                                println!("   Found {} changes", result.edits.len());
                                
                                if config.auto_fix {
                                    for edit in result.edits {
                                        edit.apply()?;
                                    }
                                    println!("   ✓ Auto-fixed");
                                } else {
                                    for edit in &result.edits {
                                        println!("   • {}", edit.message);
                                    }
                                    println!("   Run with --fix to apply");
                                }
                            }
                            Ok(_) => {
                                println!("   ✓ No issues");
                            }
                            Err(e) => {
                                println!("   ✗ Error: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Watch error: {}", e);
            }
        }
    }
}
```

### Deliverables Phase 4
- Cross-file rename/move operations (class move acid test passes)
- Native plugin system (.so/.dylib)
- Interactive mode with per-change approval
- Watch mode for continuous refactoring
- IDE integration protocol (LSP-like)
- Rename method/property with all reference updates
- Extract method refactoring
- Inline variable refactoring

---

## Phase 5: Framework Support & Ecosystem (Ongoing)

### Goals
- Laravel-specific rules
- Symfony-specific rules
- Doctrine migrations
- PHPUnit upgrades

### 5.1 Laravel Rules

```yaml
# sets/laravel11.yaml
set:
  id: laravel11
  name: Laravel 11 Upgrade
  framework: laravel
  framework_version: "11.0"

rules:
  - id: facade-to-injection
    name: Replace Facade usage with dependency injection
    description: |
      Converts static Facade calls to injected dependencies
      for better testability and explicit dependencies.
    match:
      type: static_call
      class:
        pattern: "Illuminate\\Support\\Facades\\*"
      method:
        capture: $method
    transform:
      inject_dependency: true
      pattern: "$this->{facadeAccessor}->{$method}(...)"
    
  - id: route-closure-to-controller
    name: Extract route closures to controller methods
    match:
      type: method_call
      object: Route
      method: [get, post, put, patch, delete]
      args:
        - type: string
          capture: $path
        - type: closure
          capture: $handler
    transform:
      extract_to: "App\\Http\\Controllers\\{routeToController($path)}Controller"
      method_name: "{httpMethodToAction($method)}"
    
  - id: config-to-typed-config
    name: Replace config() calls with typed config objects
    match:
      type: function_call
      name: config
      args:
        - type: string
          value:
            pattern: "^(app|database|cache|mail)\\."
            capture: $key
    transform:
      pattern: "app({configClass($key)}::class)->{configProperty($key)}"

  - id: request-validate-to-form-request
    name: Extract validation to Form Request classes
    match:
      type: method_call
      chain:
        - object: "$request"
          method: validate
          args:
            - capture: $rules
    transform:
      extract_to: "App\\Http\\Requests\\{controllerToRequest()}Request"
      inject: true
```

### 5.2 Symfony Rules

```yaml
# sets/symfony7.yaml
set:
  id: symfony7
  name: Symfony 7 Upgrade
  framework: symfony
  framework_version: "7.0"

rules:
  - id: annotation-to-attribute
    name: Convert Doctrine/Symfony annotations to PHP 8 attributes
    match:
      type: annotation
      name:
        pattern: "@(Route|Entity|Column|ORM\\*|Assert\\*)"
        capture: $annotation
    transform:
      to_attribute: true
      mapping:
        "@Route": "#[Route]"
        "@ORM\\Entity": "#[ORM\\Entity]"
        "@ORM\\Column": "#[ORM\\Column]"
        "@Assert\\NotBlank": "#[Assert\\NotBlank]"
    
  - id: container-get-to-autowire
    name: Replace container->get() with constructor injection
    match:
      type: method_call
      object:
        type_hint: "Psr\\Container\\ContainerInterface"
      method: get
      args:
        - type: class_constant
          capture: $service
    transform:
      inject_constructor: true
      remove_container_call: true
    
  - id: yaml-config-to-php
    name: Convert YAML config to PHP config
    file_pattern: "config/**/*.yaml"
    transform:
      convert_to: php
      output_pattern: "config/**/*.php"

  - id: controller-abstract-to-attribute
    name: Use controller attributes instead of AbstractController methods
    match:
      type: method_call
      object: "$this"
      method: [getParameter, getUser, isGranted]
    transform:
      getParameter:
        pattern: "#[Autowire('%{param}%')] {type} ${param}"
      getUser:
        inject: "Security $security"
        pattern: "$security->getUser()"
      isGranted:
        pattern: "#[IsGranted('{role}')]"
```

### 5.3 Doctrine Rules

```yaml
# sets/doctrine3.yaml
set:
  id: doctrine3
  name: Doctrine ORM 3.0 Upgrade
  
rules:
  - id: annotation-to-attribute-doctrine
    name: Convert Doctrine annotations to attributes
    match:
      type: annotation
      namespace: "Doctrine\\ORM\\Mapping"
    transform:
      to_attribute: true
      preserve_order: true
      
  - id: repository-interface
    name: Use EntityManagerInterface instead of EntityManager
    match:
      type: type_hint
      class: "Doctrine\\ORM\\EntityManager"
    transform:
      pattern: "Doctrine\\ORM\\EntityManagerInterface"
      
  - id: query-builder-changes
    name: Update deprecated QueryBuilder methods
    match:
      type: method_call
      object:
        type: "Doctrine\\ORM\\QueryBuilder"
      method:
        deprecated_in: "2.14"
    transform:
      mapping:
        select: setSelect
        from: setFrom
```

### 5.4 PHPUnit Rules

```yaml
# sets/phpunit11.yaml
set:
  id: phpunit11
  name: PHPUnit 11 Upgrade
  
rules:
  - id: assertion-rename
    name: Update renamed assertion methods
    match:
      type: method_call
      object: "$this"
      method:
        in: [assertFileNotExists, assertDirectoryNotExists, assertNotIsReadable]
    transform:
      mapping:
        assertFileNotExists: assertFileDoesNotExist
        assertDirectoryNotExists: assertDirectoryDoesNotExist
        assertNotIsReadable: assertIsNotReadable
        
  - id: data-provider-attribute
    name: Convert @dataProvider to #[DataProvider] attribute
    match:
      type: phpdoc
      tag: "@dataProvider"
      value:
        capture: $provider
    transform:
      remove_phpdoc_tag: true
      add_attribute: "#[DataProvider('{$provider}')]"
      
  - id: test-attribute
    name: Convert @test to #[Test] attribute
    match:
      type: phpdoc
      tag: "@test"
    transform:
      remove_phpdoc_tag: true
      add_attribute: "#[Test]"
      
  - id: mock-builder-changes
    name: Update MockBuilder method calls
    match:
      type: method_call
      chain:
        - method: getMockBuilder
        - method: setMethods
          args:
            capture: $methods
    transform:
      pattern: "->onlyMethods({$methods})"
```

### 5.5 Framework Detection

```rust
// crates/php-refactor-framework/src/detect.rs

pub enum Framework {
    Laravel { version: Version },
    Symfony { version: Version },
    None,
}

pub fn detect_framework(project_root: &Path) -> Framework {
    // Check composer.json
    let composer_path = project_root.join("composer.json");
    
    if let Ok(content) = fs::read_to_string(&composer_path) {
        if let Ok(composer) = serde_json::from_str::<ComposerJson>(&content) {
            // Check for Laravel
            if let Some(version) = composer.require.get("laravel/framework") {
                return Framework::Laravel { 
                    version: Version::parse(version) 
                };
            }
            
            // Check for Symfony
            if let Some(version) = composer.require.get("symfony/framework-bundle") {
                return Framework::Symfony { 
                    version: Version::parse(version) 
                };
            }
        }
    }
    
    Framework::None
}
```

### Deliverables Phase 5
- Laravel rule set (50+ rules)
- Symfony rule set (50+ rules)
- Doctrine ORM 3.0 migration rules
- PHPUnit 10/11 migration rules
- Framework auto-detection
- Framework-specific type stubs
- Plugin packages for each framework (installable via Composer wrapper)

---

## Timeline Summary

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| **Phase 0** | 2-3 weeks | PoC, technical design, benchmarks |
| **Phase 1** | 4-6 weeks | Core engine, basic rules, CLI |
| **Phase 2** | 3-4 weeks | YAML/TOML rules, rule sets |
| **Phase 3** | 4-6 weeks | Type awareness, PHPDoc parsing |
| **Phase 4** | 4-6 weeks | Cross-file refactoring, plugins, interactive |
| **Phase 5** | Ongoing | Framework support (Laravel, Symfony, etc.) |

**Total to MVP (Phase 0-2)**: ~10-13 weeks
**Total to Feature Parity (Phase 0-4)**: ~17-25 weeks

### Milestone Releases

| Version | Phase | Description |
|---------|-------|-------------|
| 0.1.0 | Phase 1 | Basic CLI with 10 Rust rules |
| 0.2.0 | Phase 2 | YAML/TOML rules, PHP upgrade sets |
| 0.3.0 | Phase 3 | Type-aware refactoring |
| 0.5.0 | Phase 4 | Cross-file refactoring, class move |
| 1.0.0 | Phase 4+ | Production-ready, stable API |

---

## Technical Risks & Mitigations

### Risk 1: Mago API Stability
- **Risk**: Mago crates are still evolving (currently 1.0.0-rc)
- **Mitigation**: 
  - Pin specific versions in Cargo.toml
  - Contribute upstream fixes for issues found
  - Maintain thin abstraction layer over mago crates
  - Be prepared to fork if necessary

### Risk 2: Format Preservation Complexity
- **Risk**: Maintaining exact whitespace/comments is technically challenging
- **Mitigation**: 
  - Start with "good enough" (preserve indentation level)
  - Iterate based on user feedback
  - Study how Prettier/rustfmt handle similar challenges
  - Consider hybrid approach: span edits for simple changes, full reprint for complex ones

### Risk 3: Type Inference Accuracy
- **Risk**: Mago's analyzer may not match PHPStan/Psalm accuracy
- **Mitigation**: 
  - Make type info optional (rules degrade gracefully)
  - Support PHPStan baseline import
  - Focus on high-confidence type inference first
  - Allow manual type overrides in config

### Risk 4: Rule Compatibility
- **Risk**: Users expect Rector rule compatibility
- **Mitigation**: 
  - Document differences clearly
  - Provide migration guide from Rector
  - Focus on most-used rules first (80/20 rule)
  - Consider Rector rule import tool

### Risk 5: Cross-File Refactoring Correctness
- **Risk**: Missing references during class move could break code
- **Mitigation**:
  - Comprehensive test suite (see Appendix B)
  - Dry-run mode with full preview
  - Integration with PHPStan for post-refactor verification
  - Conservative approach: warn on uncertain cases

---

## Success Metrics

1. **Performance**: 10x faster than Rector on 10k file codebase
2. **Accuracy**: <1% false positive rate on transformations
3. **Coverage**: 80% of Rector's most-used rules implemented
4. **Adoption**: Integration with 2+ major IDEs/editors
5. **Format Quality**: Diffs contain only semantic changes (no formatting noise)
6. **Class Move Test**: Pass the comprehensive class move test
   - All 28+ reference types correctly updated
   - Implicit namespace resolution → adds use statement
   - Group use statements properly split
   - String FQNs in configs detected
   - PHPDoc references (including generics) updated
   - require/include paths updated

---

## Getting Started

```bash
# 1. Set up development environment
git clone https://github.com/carthage-software/mago.git
cd mago
cargo build

# 2. Create new project
cargo new --lib php-refactor
cd php-refactor

# 3. Add mago dependencies
cat >> Cargo.toml << 'EOF'
[dependencies]
mago-syntax = { git = "https://github.com/carthage-software/mago" }
mago-ast = { git = "https://github.com/carthage-software/mago" }
mago-span = { git = "https://github.com/carthage-software/mago" }
mago-source = { git = "https://github.com/carthage-software/mago" }
mago-fixer = { git = "https://github.com/carthage-software/mago" }
rayon = "1.8"
EOF

# 4. Start with Phase 0 PoC
```

---

## Resources

- [Mago GitHub](https://github.com/carthage-software/mago)
- [Mago Documentation](https://mago.carthage.software/)
- [Rector Rule Reference](https://github.com/rectorphp/rector/blob/main/docs/rector_rules_overview.md)
- [ast-grep (for pattern inspiration)](https://ast-grep.github.io/)
- [OXC (similar Rust toolchain for JS)](https://github.com/oxc-project/oxc)
- [tree-sitter (for CST concepts)](https://tree-sitter.github.io/tree-sitter/)

---

## Appendix A: Phase 0 Quick Start Guide

### Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Useful tools
cargo install cargo-watch  # For auto-rebuild
cargo install just         # Task runner (optional)
```

### Study Mago Internals

```bash
# Clone mago
git clone https://github.com/carthage-software/mago.git ~/projects/mago
cd ~/projects/mago
cargo build --release
```

**Key files to study:**

```
crates/
├── mago-syntax/src/         # Parser implementation
├── mago-fixer/src/          # ⚠️ Study this first - fix application logic
├── mago-linter/src/         # How rules are structured
└── mago-span/src/           # Source position handling
```

### Create PoC Project

**Cargo.toml:**

```toml
[package]
name = "php-refactor-poc"
version = "0.1.0"
edition = "2021"

[dependencies]
mago-syntax = { git = "https://github.com/carthage-software/mago", branch = "main" }
mago-source = { git = "https://github.com/carthage-software/mago", branch = "main" }
mago-span = { git = "https://github.com/carthage-software/mago", branch = "main" }
mago-interner = { git = "https://github.com/carthage-software/mago", branch = "main" }
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }
colored = "2.0"
walkdir = "2.4"
diff = "0.1"

[dev-dependencies]
pretty_assertions = "1.4"
```

**src/main.rs:**

```rust
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod rules;
mod editor;

#[derive(Parser)]
#[command(name = "php-refactor-poc")]
struct Cli {
    #[arg(required = true)]
    paths: Vec<PathBuf>,
    
    #[arg(long, short = 'n')]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    for path in &cli.paths {
        process_path(path, cli.dry_run)?;
    }
    
    Ok(())
}

fn process_path(path: &std::path::Path, dry_run: bool) -> Result<()> {
    if path.is_file() {
        return process_file(path, dry_run);
    }
    
    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "php"))
    {
        process_file(entry.path(), dry_run)?;
    }
    
    Ok(())
}

fn process_file(path: &std::path::Path, dry_run: bool) -> Result<()> {
    let source_code = std::fs::read_to_string(path)?;
    
    let interner = mago_interner::ThreadedInterner::new();
    let source_id = interner.intern(path.to_string_lossy().as_ref());
    let source = mago_source::Source::new(source_id, source_code.clone());
    
    let result = mago_syntax::parse(&source);
    
    if !result.errors.is_empty() {
        eprintln!("Parse errors in {:?}, skipping", path);
        return Ok(());
    }
    
    let edits = rules::array_push::check(&result.program, &source_code);
    
    if edits.is_empty() {
        return Ok(());
    }
    
    println!("{}:", path.display());
    
    let new_source = editor::apply_edits(&source_code, &edits);
    
    if dry_run {
        print_diff(&source_code, &new_source);
    } else {
        std::fs::write(path, new_source)?;
        println!("  Applied {} changes", edits.len());
    }
    
    Ok(())
}

fn print_diff(old: &str, new: &str) {
    use colored::*;
    for d in diff::lines(old, new) {
        match d {
            diff::Result::Left(l) => println!("{}", format!("- {}", l).red()),
            diff::Result::Right(r) => println!("{}", format!("+ {}", r).green()),
            diff::Result::Both(_, _) => {}
        }
    }
}
```

**src/editor.rs:**

```rust
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

pub fn apply_edits(source: &str, edits: &[Edit]) -> String {
    if edits.is_empty() {
        return source.to_string();
    }
    
    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by(|a, b| b.span.start.offset.cmp(&a.span.start.offset));
    
    let mut result = source.to_string();
    
    for edit in sorted {
        let start = edit.span.start.offset as usize;
        let end = edit.span.end.offset as usize;
        result.replace_range(start..end, &edit.replacement);
    }
    
    result
}
```

**src/rules/mod.rs:**

```rust
pub mod array_push;
```

**src/rules/array_push.rs:**

```rust
//! Rule: Convert array_push($arr, $val) to $arr[] = $val

use mago_syntax::ast::*;
use mago_span::HasSpan;
use crate::editor::Edit;

pub fn check(program: &Program, source: &str) -> Vec<Edit> {
    let mut edits = Vec::new();
    visit_statements(&program.statements, source, &mut edits);
    edits
}

fn visit_statements(stmts: &[Statement], source: &str, edits: &mut Vec<Edit>) {
    for stmt in stmts {
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
        // ... handle other statement types
        _ => {}
    }
}

fn check_expression(expr: &Expression, source: &str, edits: &mut Vec<Edit>) {
    if let Expression::Call(call) = expr {
        if let Expression::Identifier(ident) = &*call.target {
            let name = &source[ident.span().start.offset as usize..ident.span().end.offset as usize];
            
            if name == "array_push" {
                if let Some(args) = &call.arguments {
                    let arg_list: Vec<_> = args.arguments.iter().collect();
                    
                    if arg_list.len() == 2 {
                        let arr = &source[arg_list[0].span().start.offset as usize..arg_list[0].span().end.offset as usize];
                        let val = &source[arg_list[1].span().start.offset as usize..arg_list[1].span().end.offset as usize];
                        
                        edits.push(Edit::new(
                            call.span(),
                            format!("{}[] = {}", arr, val),
                            "Replace array_push() with short syntax",
                        ));
                    }
                }
            }
        }
    }
}
```

### Test the PoC

```php
<?php
// test.php
$items = [];
array_push($items, 'foo');
array_push($items, $bar);
```

```bash
cargo run -- --dry-run test.php
```

---

## Appendix B: Class Move Test Case (Acid Test)

### Test Scenario

Move `App\Legacy\Services\PaymentProcessor` to `App\Payment\Processing\PaymentProcessor`

### Files Involved

```
src/
├── Legacy/Services/PaymentProcessor.php     # Class to move
├── Controllers/CheckoutController.php       # Simple use statement
├── Services/OrderService.php                # Aliased import
├── Models/Invoice.php                       # FQN type hints
├── Events/PaymentCompleted.php              # PHPDoc only
├── Tests/PaymentProcessorTest.php           # Multiple reference types
├── config/services.php                      # String FQN in config
├── bootstrap.php                            # require_once path
├── Legacy/Services/PaymentLogger.php        # Implicit namespace (EDGE CASE)
└── Services/PaymentFacade.php               # Grouped use statement
```

### Reference Types Tested (28 types)

| # | Reference Type | File | Complexity |
|---|----------------|------|------------|
| 1 | Use statement (simple) | CheckoutController.php | Simple |
| 2 | Use statement (aliased) | OrderService.php | Medium |
| 3 | Use statement (grouped) | PaymentFacade.php | Medium |
| 4 | FQN type hint (property) | Invoice.php | Simple |
| 5 | FQN type hint (parameter) | Invoice.php | Simple |
| 6 | FQN type hint (return) | Invoice.php | Simple |
| 7 | FQN type hint (nullable) | Invoice.php | Simple |
| 8 | FQN instantiation | Invoice.php | Simple |
| 9 | Short type hint (via use) | CheckoutController.php | Simple |
| 10 | Short instantiation (via use) | PaymentProcessorTest.php | Simple |
| 11 | Static method call | PaymentProcessorTest.php | Simple |
| 12 | Constant access | PaymentProcessorTest.php | Simple |
| 13 | ::class constant | PaymentProcessorTest.php | Simple |
| 14 | instanceof | PaymentProcessorTest.php | Simple |
| 15 | PHPDoc @param | PaymentCompleted.php | Medium |
| 16 | PHPDoc @return | PaymentCompleted.php | Medium |
| 17 | PHPDoc @var | PaymentCompleted.php | Medium |
| 18 | PHPDoc @see | PaymentCompleted.php | Medium |
| 19 | PHPDoc generic | (Collection&lt;Class&gt;) | Hard |
| 20 | String FQN (::class) | services.php | Hard |
| 21 | String FQN (literal) | services.php | Hard |
| 22 | require_once path | bootstrap.php | Hard |
| 23 | Namespace declaration | PaymentProcessor.php | Simple |
| 24 | **Implicit namespace** | PaymentLogger.php | **HARD** |
| 25 | PHP 8 Attribute | Various | Medium |
| 26 | catch clause | Various | Simple |
| 27 | extends clause | Various | Simple |
| 28 | implements clause | Various | Simple |

### Edge Case: Implicit Namespace Resolution

**Before (PaymentLogger.php in same namespace):**

```php
<?php
namespace App\Legacy\Services;

// NO use statement - PaymentProcessor resolves from same namespace
class PaymentLogger {
    public function log(PaymentProcessor $processor): void { }
}
```

**After (must ADD use statement):**

```php
<?php
namespace App\Legacy\Services;

use App\Payment\Processing\PaymentProcessor;  // ← ADDED

class PaymentLogger {
    public function log(PaymentProcessor $processor): void { }
}
```

### Success Criteria

```bash
php-refactor move-class \
    --from src/Legacy/Services/PaymentProcessor.php \
    --to src/Payment/Processing/ \
    --namespace "App\Payment\Processing"

# After running:
# ✅ PHPStan passes at same level
# ✅ All existing tests pass  
# ✅ No broken references
# ✅ Only modified lines in git diff (format preservation)
# ✅ 23+ edits across 10 files
```

---

*End of Complete Project Plan*
