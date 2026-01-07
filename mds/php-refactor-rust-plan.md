# PHPRefactor: A Rust-based Rector Alternative

## Project Codename: `php-refactor` (or `refactor-php`, `phpmod`, `transmute`)

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
        // Apply to replacement
        // This is the key to format preservation
        todo!()
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

### 2.1 Declarative Rule Format

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

### 2.2 Rule Configuration Schema

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
    // ... more node types
}

#[derive(Debug, Deserialize)]
pub struct TransformDef {
    pub pattern: Option<String>,      // Template string
    pub remove: Option<Vec<String>>,  // Captures to remove
    pub modify: Option<Vec<ModifyDef>>,
}
```

### 2.3 Rule Sets (Like Rector's SetLists)

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
```

### Deliverables Phase 2
- TOML/YAML rule parser
- 20+ declarative rules for PHP upgrades
- Rule sets for PHP 7.4→8.0, 8.0→8.1, 8.1→8.2, 8.2→8.3, 8.3→8.4
- Project configuration file support
- `php-refactor init` command

---

## Phase 3: Type-Aware Refactoring (4-6 weeks)

### Goals
- Integrate type inference from mago-analyzer
- PHPDoc parsing for legacy type info
- Type-dependent rule transformations

### 3.1 Type System Integration

```rust
// crates/php-refactor-types/src/lib.rs

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
        // ... type compatibility check
    }
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

/// Parses PHPDoc blocks for type information
pub struct PhpDocParser;

impl PhpDocParser {
    pub fn parse_var_type(&self, doc: &str) -> Option<Type> {
        // Parse @var annotations
        // Handle complex types: array<string, int>, Collection<User>
    }
    
    pub fn parse_param_types(&self, doc: &str) -> HashMap<String, Type> {
        // Parse @param annotations
    }
    
    pub fn parse_return_type(&self, doc: &str) -> Option<Type> {
        // Parse @return annotation
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

# Type condition!
conditions:
  - capture: $haystack
    type: string
  - capture: $needle  
    type: string

transform:
  pattern: "str_contains({$haystack}, {$needle})"
```

### Deliverables Phase 3
- Type inference integration
- PHPDoc parser
- 15+ type-aware rules
- Type info available in declarative rule conditions
- Add return type declarations based on inference

---

## Phase 4: Advanced Features (4-6 weeks)

### Goals
- Cross-file refactoring
- Rename/move with reference updates
- Custom Rust rule plugin system

### 4.1 Cross-File Operations

**KEY MILESTONE: Class Move Refactoring (see `php-refactor-class-move-test.md`)**

This is the "acid test" for the refactoring tool — moving a class to a different namespace while updating ALL references across the codebase.

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

// Rename class and update all references
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
        let has_implicit = refs.iter().any(|r| r.kind == ReferenceKind::ImplicitNamespace);
        
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

/// Trait for dynamically loaded rule plugins
pub trait RulePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn rules(&self) -> Vec<Box<dyn Rule>>;
}

/// Load plugin from shared library
pub fn load_plugin(path: &Path) -> Result<Box<dyn RulePlugin>> {
    unsafe {
        let lib = libloading::Library::new(path)?;
        let create_fn: libloading::Symbol<fn() -> Box<dyn RulePlugin>> = 
            lib.get(b"create_plugin")?;
        Ok(create_fn())
    }
}

// Plugin implementation (in separate crate)
#[no_mangle]
pub fn create_plugin() -> Box<dyn RulePlugin> {
    Box::new(MyCustomPlugin::new())
}
```

### 4.3 Interactive Mode

```rust
// CLI interactive refactoring
pub async fn interactive_refactor(engine: &RefactorEngine) {
    let edits = engine.run_dry();
    
    for edit in edits {
        println!("{}", edit.diff_preview());
        
        print!("Apply this change? [y/n/q/a] ");
        match read_input().await {
            'y' => apply_single(edit),
            'n' => skip(edit),
            'q' => break,
            'a' => {
                apply_all_remaining(edits);
                break;
            }
        }
    }
}
```

### Deliverables Phase 4
- Cross-file rename/move operations
- Native plugin system (.so/.dylib)
- Interactive mode
- Watch mode for continuous refactoring
- IDE integration protocol (LSP-like)

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
rules:
  - id: facade-to-injection
    description: "Replace Facade usage with dependency injection"
    
  - id: route-closure-to-controller
    description: "Extract route closures to controller methods"
    
  - id: config-to-typed-config
    description: "Replace config() calls with typed config objects"
```

### 5.2 Symfony Rules

```yaml
# sets/symfony7.yaml  
rules:
  - id: annotation-to-attribute
    description: "Convert Doctrine/Symfony annotations to PHP 8 attributes"
    
  - id: container-get-to-autowire
    description: "Replace container->get() with constructor injection"
```

---

## Timeline Summary

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| **Phase 0** | 2-3 weeks | PoC, technical design |
| **Phase 1** | 4-6 weeks | Core engine, basic rules, CLI |
| **Phase 2** | 3-4 weeks | YAML/TOML rules, rule sets |
| **Phase 3** | 4-6 weeks | Type awareness, PHPDoc |
| **Phase 4** | 4-6 weeks | Cross-file, plugins, interactive |
| **Phase 5** | Ongoing | Framework support |

**Total to MVP (Phase 0-2)**: ~10-13 weeks
**Total to Feature Parity (Phase 0-4)**: ~17-25 weeks

---

## Technical Risks & Mitigations

### Risk 1: Mago API Stability
- **Risk**: Mago crates are still evolving
- **Mitigation**: Pin specific versions, contribute upstream fixes, maintain fork if needed

### Risk 2: Format Preservation Complexity
- **Risk**: Maintaining exact whitespace/comments is hard
- **Mitigation**: Start with "good enough" (preserve indentation level), iterate

### Risk 3: Type Inference Accuracy
- **Risk**: Mago's analyzer may not match PHPStan/Psalm accuracy
- **Mitigation**: Make type info optional, graceful degradation

### Risk 4: Rule Compatibility
- **Risk**: Users expect Rector rule compatibility
- **Mitigation**: Document differences, provide migration guide, focus on common rules first

---

## Success Metrics

1. **Performance**: 10x faster than Rector on 10k file codebase
2. **Accuracy**: <1% false positive rate on transformations
3. **Coverage**: 80% of Rector's most-used rules implemented
4. **Adoption**: Integration with 2+ major IDEs/editors
5. **Format Quality**: Diffs contain only semantic changes (no formatting noise)
6. **Class Move Test**: Pass the comprehensive class move test (see `php-refactor-class-move-test.md`)
   - All 28+ reference types correctly updated
   - Implicit namespace resolution → adds use statement
   - Group use statements properly split
   - String FQNs in configs detected
   - PHPDoc references (including generics) updated
   - require/include paths updated

---

## Getting Started (Your First Steps)

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

---

## Project Documents

| Document | Purpose |
|----------|---------|
| `php-refactor-rust-plan.md` | This file - main project plan |
| `php-refactor-phase0-quickstart.md` | Phase 0 implementation guide with code |
| `php-refactor-class-move-test.md` | Comprehensive class move test case (acid test) |

---

## Appendix: Class Move Test Summary

The class move test (`php-refactor-class-move-test.md`) is the "acid test" for the tool. It validates:

### Test Scenario
Move `App\Legacy\Services\PaymentProcessor` to `App\Payment\Processing\PaymentProcessor`

### Files Affected (10 files, 23+ edits)
1. **PaymentProcessor.php** - Namespace declaration, file move
2. **CheckoutController.php** - Simple use statement update
3. **OrderService.php** - Aliased use statement (`as Processor`)
4. **Invoice.php** - FQN type hints and PHPDoc
5. **PaymentCompleted.php** - PHPDoc only references
6. **PaymentProcessorTest.php** - Multiple reference types
7. **services.php** - ::class and string FQN in config
8. **bootstrap.php** - require_once path + FQN
9. **PaymentLogger.php** - Implicit namespace → must ADD use statement
10. **PaymentFacade.php** - Group use statement → must split

### Reference Types Tested (28 types)
- Use statements (simple, aliased, grouped)
- Type hints (param, return, property, nullable, union, intersection)
- Instantiation, static access, constants
- instanceof, catch, extends, implements
- PHPDoc (@param, @return, @var, @throws, @see, generics)
- String literals, attributes, file includes
- **Edge case**: Implicit same-namespace resolution
