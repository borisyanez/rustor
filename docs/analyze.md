# Static Analysis (`rustor analyze`)

Rustor includes a PHPStan-compatible static analysis engine that detects errors, type issues, and potential bugs in PHP code without executing it.

## Overview

The `rustor analyze` command provides:

- **PHPStan-compatible analysis** at levels 0-6
- **NEON configuration file support** - use your existing `phpstan.neon`
- **Baseline support** for gradual adoption
- **Multiple output formats** - raw, JSON, table, GitHub Actions
- **Parallel processing** for fast analysis of large codebases
- **PHPStan exact compatibility mode** for CI/CD integration

## Quick Start

```bash
# Basic analysis at level 0
rustor analyze src/

# Analysis at level 3 (return types, property types)
rustor analyze src/ --level 3

# Use existing PHPStan configuration
rustor analyze -c phpstan.neon

# JSON output for CI
rustor analyze src/ --level 2 --error-format json

# GitHub Actions annotations
rustor analyze src/ --level 2 --error-format github
```

## Configuration

### Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--configuration <FILE>` | `-c` | PHPStan config file (phpstan.neon) |
| `--level <LEVEL>` | `-l` | Analysis level (0-9, max) |
| `--error-format <FORMAT>` | | Output format: raw, json, table, github |
| `--generate-baseline <FILE>` | | Generate baseline file |
| `--baseline <FILE>` | | Use baseline to filter issues |
| `--phpstan-compat` | | PHPStan exact compatibility mode |
| `--verbose` | `-v` | Verbose output |

### PHPStan Configuration File (NEON)

Rustor fully supports PHPStan's NEON configuration format. Create a `phpstan.neon` file in your project root:

```neon
# phpstan.neon
parameters:
    level: 3
    paths:
        - src/
        - app/
    excludePaths:
        - vendor/
        - tests/fixtures/
    phpVersion: 80100  # PHP 8.1.0
    treatPhpDocTypesAsCertain: true
    checkMissingTypehints: false
    reportUnmatchedIgnoredErrors: true

includes:
    - phpstan-baseline.neon
```

### Configuration Parameters Reference

#### Core Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `level` | int/string | 0 | Analysis level (0-9 or "max") |
| `paths` | string[] | [] | Paths to analyze |
| `excludePaths` | string[]/object | [] | Paths to exclude from analysis |
| `phpVersion` | int | null | PHP version as integer (80100 = PHP 8.1.0) |

#### Analysis Behavior

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `treatPhpDocTypesAsCertain` | bool | true | Trust PHPDoc type annotations |
| `checkMissingTypehints` | bool | false | Report missing type declarations (level 6) |
| `reportUnmatchedIgnoredErrors` | bool | true | Report when ignoreErrors patterns don't match |

#### Performance

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `parallel.maximumNumberOfProcesses` | int | auto | Max parallel processes |

#### Error Ignoring

| Parameter | Type | Description |
|-----------|------|-------------|
| `ignoreErrors` | array | Patterns to ignore (see below) |

### Ignoring Errors

You can suppress specific errors using the `ignoreErrors` parameter:

```neon
parameters:
    ignoreErrors:
        # Simple string match
        - 'Call to undefined function legacy_function'

        # Regex pattern (starts with # or /)
        - '#Call to undefined method.*Legacy#'

        # Complex ignore with path filter
        -
            message: '#Variable \$data might not be defined#'
            path: src/Legacy/*.php

        # Ignore by identifier
        -
            identifier: variable.undefined
            path: src/Generated/*.php

        # Ignore with count limit
        -
            message: '#Undefined variable#'
            count: 5  # Allow up to 5 matches
```

### Exclude Paths

Multiple formats are supported:

```neon
parameters:
    # Simple array
    excludePaths:
        - vendor/
        - tests/fixtures/
        - *.generated.php

    # Object format with analyse/analyseAndScan
    excludePaths:
        analyse:
            - generated/
        analyseAndScan:
            - vendor/

    # Alternative key names (for compatibility)
    excludes_analyse:
        - old_code/
```

### Include Files

Include other configuration files:

```neon
includes:
    - phpstan-baseline.neon
    - vendor/phpstan/phpstan-strict-rules/rules.neon
    - custom-rules.neon
```

### Stub Files

Provide type information for code that can't be analyzed:

```neon
parameters:
    stubFiles:
        - stubs/legacy-functions.stub
        - stubs/external-library.stub
```

### Bootstrap Files

PHP files to execute before analysis:

```neon
parameters:
    bootstrapFiles:
        - phpstan-bootstrap.php
```

---

## Analysis Levels

Rustor supports PHPStan analysis levels 0-6, with each level adding more checks:

### Level 0 - Basic Checks

Catches obvious errors that would cause runtime failures.

| Check | PHPStan ID | Description |
|-------|------------|-------------|
| Undefined function | `function.notFound` | `undefined_function()` |
| Undefined class | `class.notFound` | `new UndefinedClass()` |
| Undefined static method | `staticMethod.notFound` | `Foo::undefinedMethod()` |
| Undefined class constant | `classConstant.notFound` | `Foo::UNDEFINED_CONST` |
| Argument count | `arguments.count` | Wrong number of arguments |
| Missing return | `return.missing` | Function declares return type but has no return |

### Level 1 - Variable Analysis

Adds variable scope tracking and magic method warnings.

| Check | PHPStan ID | Description |
|-------|------------|-------------|
| Undefined variable | `variable.undefined` | `echo $undefined;` |
| Possibly undefined | `variable.possiblyUndefined` | Variable only defined in if branch |
| Unused constructor param | `constructor.unusedParameter` | Constructor parameter never used |

### Level 2 - Type-Aware Checks

Requires type tracking through variable assignments.

| Check | PHPStan ID | Description |
|-------|------------|-------------|
| Undefined method | `method.notFound` | `$obj->undefinedMethod()` |
| Undefined property | `property.notFound` | `$obj->undefinedProp` |
| Method argument count | `arguments.count` | Wrong args to method calls |

### Level 3 - Return Types & Property Types

Validates return types and property assignments.

| Check | PHPStan ID | Description |
|-------|------------|-------------|
| Return type mismatch | `return.type` | `function foo(): string { return 42; }` |
| Property type mismatch | `property.type` | `$this->intProp = "string";` |
| Void return | `return.void` | `function foo(): void { return $x; }` |

### Level 4 - Dead Code Detection

Identifies unreachable and redundant code.

| Check | PHPStan ID | Description |
|-------|------------|-------------|
| Unreachable code | `deadCode.unreachable` | Code after return/throw |
| Always-false instanceof | `instanceof.alwaysFalse` | `$string instanceof stdClass` |
| Redundant type check | `function.alreadyNarrowedType` | `is_string($knownString)` |
| Unused function result | `function.resultUnused` | `strlen($s);` (result discarded) |

### Level 5 - Argument Types

Validates argument types in function and method calls.

| Check | PHPStan ID | Description |
|-------|------------|-------------|
| Argument type mismatch | `argument.type` | `expectsString(42)` |

### Level 6 - Missing Typehints

Reports missing type declarations.

| Check | PHPStan ID | Description |
|-------|------------|-------------|
| Missing parameter type | `missingType.parameter` | `function foo($x)` |
| Missing return type | `missingType.return` | `function foo() {}` |
| Missing property type | `missingType.property` | `public $prop;` |

---

## PHPStan Compatibility Mode

By default, rustor has some lenient behaviors that differ from PHPStan:

- Classes with `__get` magic method: rustor issues **warnings** for undefined property access, while PHPStan issues **errors**

Use `--phpstan-compat` to enable strict PHPStan compatibility:

```bash
# Default mode (lenient)
rustor analyze src/ --level 1

# PHPStan exact compatibility mode
rustor analyze src/ --level 1 --phpstan-compat
```

### Compatibility Status

| Level | Match Rate | Status |
|-------|------------|--------|
| 0 | 100% (13/13) | Full compatibility |
| 1 | 100% (13/13) | Full compatibility |
| 2 | 100% (13/13) | Full compatibility |
| 3 | 100% (13/13) | Full compatibility |
| 4 | 100% (13/13) | Full compatibility |
| 5 | 100% (13/13) | Full compatibility |
| 6 | 100% (13/13) | Full compatibility |

---

## Output Formats

Rustor supports four output formats, all designed to match PHPStan's output exactly.

### Table (default)

PHPStan-style formatted table with line numbers, messages, and error identifiers:

```bash
rustor analyze src/ --level 3 --error-format table
```

Output:

```
 ------ -----------------------------------------------------------------------
  Line   Controller.php
 ------ -----------------------------------------------------------------------
  45     Call to undefined function undefined_func()
         🪪  function.notFound
  67     Undefined variable $request
         🪪  variable.undefined
 ------ -----------------------------------------------------------------------

 [ERROR] Found 2 errors
```

When there are no errors:

```
 [OK] No errors
```

### Raw

Simple `file:line:message` format, one error per line. Matches PHPStan's raw format exactly:

```bash
rustor analyze src/ --level 3 --error-format raw
```

Output:

```
src/Controller.php:45:Call to undefined function undefined_func()
src/Controller.php:67:Undefined variable $request
```

### JSON

Machine-readable JSON for CI/CD integration. Matches PHPStan's JSON output structure:

```bash
rustor analyze src/ --level 2 --error-format json
```

Output structure:

```json
{
  "totals": {
    "errors": 0,
    "file_errors": 5
  },
  "files": {
    "src/Controller.php": {
      "errors": 2,
      "messages": [
        {
          "message": "Call to undefined function undefined_func()",
          "line": 45,
          "ignorable": true,
          "identifier": "function.notFound"
        },
        {
          "message": "Undefined variable $request",
          "line": 67,
          "ignorable": true,
          "identifier": "variable.undefined"
        }
      ]
    }
  },
  "errors": []
}
```

### GitHub Actions

GitHub Actions annotations for PR checks:

```bash
rustor analyze src/ --level 2 --error-format github
```

Output:

```
::error file=src/Controller.php,line=45::Call to undefined function undefined_func()
::error file=src/Controller.php,line=67::Undefined variable $request
```

### Format Comparison

| Format | Use Case | PHPStan Compatible |
|--------|----------|-------------------|
| `table` | Terminal output, human review | Yes |
| `raw` | Simple parsing, logs | Yes |
| `json` | CI/CD, automation, scripts | Yes |
| `github` | GitHub Actions annotations | Yes |

---

## Baseline Support

Baselines allow gradual adoption by tracking existing issues and only reporting new ones.

### Generate Baseline

```bash
# Generate baseline file
rustor analyze src/ --level 3 --generate-baseline baseline.neon
```

Output format (NEON):

```neon
parameters:
    ignoreErrors:
        -
            message: '#Call to undefined function legacy_func\(\)#'
            path: src/Legacy/Helper.php
            count: 1
        -
            message: '#Undefined variable \$data#'
            path: src/Controller.php
            count: 3
```

### Use Baseline

```bash
# Only report new issues not in baseline
rustor analyze src/ --level 3 --baseline baseline.neon
```

### Include in Configuration

```neon
# phpstan.neon
includes:
    - baseline.neon

parameters:
    level: 3
    paths:
        - src/
```

---

## Programmatic Usage

```rust
use rustor_analyze::{Analyzer, config::PhpStanConfig, output::OutputFormat};
use std::path::Path;

// Load configuration
let config = PhpStanConfig::load(Path::new("phpstan.neon"))?;

// Or create with defaults
let mut config = PhpStanConfig::default();
config.level = config::Level::Level3;
config.paths.push(PathBuf::from("src/"));

// Create analyzer
let analyzer = Analyzer::new(config);

// Analyze paths
let issues = analyzer.analyze_paths(&[Path::new("src/")])?;

// Format output
let output = rustor_analyze::output::format_issues(&issues, OutputFormat::Json);
println!("{}", output);
```

### API Reference

#### `PhpStanConfig`

```rust
pub struct PhpStanConfig {
    /// Analysis level (0-9)
    pub level: Level,
    /// Paths to analyze
    pub paths: Vec<PathBuf>,
    /// Paths to exclude
    pub exclude_paths: Vec<PathBuf>,
    /// PHP version (e.g., 80100 for PHP 8.1.0)
    pub php_version: Option<u32>,
    /// Errors to ignore
    pub ignore_errors: Vec<IgnoreError>,
    /// Included config files
    pub includes: Vec<PathBuf>,
    /// Trust PHPDoc types
    pub treat_phpdoc_types_as_certain: bool,
    /// Check missing typehints
    pub check_missing_typehints: bool,
    /// Report unmatched ignored errors
    pub report_unmatched_ignored_errors: bool,
    /// Parallel processing threads
    pub parallel_max_processes: Option<usize>,
    /// Memory limit
    pub memory_limit: Option<String>,
    /// Custom rule paths
    pub custom_rule_paths: Vec<PathBuf>,
    /// Stub files
    pub stub_files: Vec<PathBuf>,
    /// Bootstrap files
    pub bootstrap_files: Vec<PathBuf>,
    /// PHPStan exact compatibility mode
    pub phpstan_compat: bool,
}
```

#### `IgnoreError`

```rust
pub struct IgnoreError {
    /// Message pattern (regex or exact match)
    pub message: String,
    /// Optional path pattern
    pub path: Option<String>,
    /// Whether this is a regex pattern
    pub is_regex: bool,
    /// Count limit (None = unlimited)
    pub count: Option<usize>,
    /// Identifier pattern
    pub identifier: Option<String>,
}
```

#### `Analyzer`

```rust
impl Analyzer {
    /// Create analyzer with configuration
    pub fn new(config: PhpStanConfig) -> Self;

    /// Create with default configuration
    pub fn with_defaults() -> Self;

    /// Set analysis level
    pub fn set_level(&mut self, level: Level);

    /// Get current configuration
    pub fn config(&self) -> &PhpStanConfig;

    /// Analyze a single file
    pub fn analyze_file(&self, path: &Path) -> Result<IssueCollection, AnalyzeError>;

    /// Analyze source code with a given path
    pub fn analyze_source(&self, path: &Path, source: &str) -> Result<IssueCollection, AnalyzeError>;

    /// Analyze multiple paths (files or directories)
    pub fn analyze_paths(&self, paths: &[&Path]) -> Result<IssueCollection, AnalyzeError>;

    /// Analyze paths from configuration
    pub fn analyze_configured_paths(&self) -> Result<IssueCollection, AnalyzeError>;
}
```

---

## CI/CD Integration

### GitHub Actions

```yaml
name: Static Analysis

on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run rustor analyze
        run: |
          rustor analyze src/ --level 3 --error-format github
```

### GitLab CI

```yaml
analyze:
  script:
    - rustor analyze src/ --level 3 --error-format json > analysis.json
  artifacts:
    reports:
      codequality: analysis.json
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Get staged PHP files
STAGED=$(git diff --cached --name-only --diff-filter=ACM | grep '\.php$')

if [ -n "$STAGED" ]; then
    rustor analyze $STAGED --level 2
    if [ $? -ne 0 ]; then
        echo "Static analysis failed. Fix errors before committing."
        exit 1
    fi
fi
```

---

## Implemented Checks Reference

### Level 0

| Check | ID | Example Error |
|-------|-----|---------------|
| Undefined function | `function.notFound` | Call to undefined function foo() |
| Undefined class | `class.notFound` | Class UndefinedClass not found |
| Undefined static method | `staticMethod.notFound` | Call to an undefined static method Foo::bar() |
| Undefined class constant | `classConstant.notFound` | Access to undefined constant Foo::CONST |
| Argument count | `arguments.count` | Function foo() invoked with 1 parameter, 2 required |
| Missing return | `return.missing` | Function foo() should return int but return statement is missing |

### Level 1

| Check | ID | Example Error |
|-------|-----|---------------|
| Undefined variable | `variable.undefined` | Undefined variable $foo |
| Possibly undefined | `variable.possiblyUndefined` | Variable $foo might not be defined |
| Unused parameter | `constructor.unusedParameter` | Constructor of class Foo has an unused parameter $bar |

### Level 2

| Check | ID | Example Error |
|-------|-----|---------------|
| Undefined method | `method.notFound` | Call to an undefined method Foo::bar() |
| Undefined property | `property.notFound` | Access to an undefined property Foo::$bar |

### Level 3

| Check | ID | Example Error |
|-------|-----|---------------|
| Return type mismatch | `return.type` | Method Foo::bar() should return string but returns int |
| Property type mismatch | `property.type` | Property Foo::$bar (int) does not accept string |
| Void return | `return.void` | Method Foo::bar() with return type void returns int |

### Level 4

| Check | ID | Example Error |
|-------|-----|---------------|
| Unreachable code | `deadCode.unreachable` | Unreachable statement - code above always terminates |
| Always-false instanceof | `instanceof.alwaysFalse` | Instanceof between string and stdClass will always evaluate to false |
| Redundant type check | `function.alreadyNarrowedType` | Call to function is_string() with string will always evaluate to true |
| Unused result | `function.resultUnused` | Call to function strlen() on a separate line has no effect |
| Always-true comparison | `smallerOrEqual.alwaysTrue` | Comparison operation "<=" between int<min, 0> and 0 is always true |

### Level 5

| Check | ID | Example Error |
|-------|-----|---------------|
| Argument type | `argument.type` | Parameter #1 $foo of method Bar::baz() expects string, int given |

### Level 6

| Check | ID | Example Error |
|-------|-----|---------------|
| Missing parameter type | `missingType.parameter` | Method Foo::bar() has parameter $x with no type specified |
| Missing return type | `missingType.return` | Method Foo::bar() has no return type specified |
| Missing property type | `missingType.property` | Property Foo::$bar has no type specified |
| Missing iterable value type | `missingType.iterableValue` | Method Foo::bar() has parameter $arr with no value type specified in iterable type array |

---

## Migration from PHPStan

If you're already using PHPStan, migration is straightforward:

1. **Use your existing config**: `rustor analyze -c phpstan.neon`
2. **Use your existing baseline**: `rustor analyze --baseline phpstan-baseline.neon`
3. **For exact compatibility**: Add `--phpstan-compat` flag

### Differences from PHPStan

| Feature | PHPStan | Rustor |
|---------|---------|--------|
| Language | PHP | Rust |
| Speed | Fast | Faster (parallel by default) |
| Memory | ~1GB for large codebases | Lower footprint |
| Levels 7-9 | Supported | Not yet implemented |
| Extensions | Many available | Core checks only |
| Magic methods | Always errors | Warnings (unless --phpstan-compat) |

---

## Troubleshooting

### "No paths configured for analysis"

Specify paths on command line or in configuration:

```bash
rustor analyze src/
# or
rustor analyze -c phpstan.neon  # with paths in config
```

### "Class X not found" for vendor classes

Ensure vendor directory is accessible or use stub files:

```neon
parameters:
    stubFiles:
        - stubs/vendor-types.stub
```

### Different results than PHPStan

Use `--phpstan-compat` for exact compatibility:

```bash
rustor analyze src/ --level 3 --phpstan-compat
```

### Memory issues with large codebases

The parallel processing is automatic. For very large codebases, you can limit parallelism:

```neon
parameters:
    parallel:
        maximumNumberOfProcesses: 4
```

---

## See Also

- [CLI Reference](cli.md) - All command-line options
- [Configuration](configuration.md) - rustor.toml configuration
- [PHPStan Documentation](https://phpstan.org/user-guide/getting-started) - PHPStan reference
