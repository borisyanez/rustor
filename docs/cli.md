# CLI Reference

Complete reference for all rustor command-line options.

## Synopsis

```bash
rustor [OPTIONS] [PATHS]...
```

## Description

Rustor is a PHP refactoring tool that analyzes PHP files and suggests or applies code transformations. By default, it runs in check mode (dry-run), showing what changes would be made without modifying files.

## Arguments

### `PATHS`

Files or directories to process. Required unless using `--list-rules`, `--staged`, or `--since`.

```bash
# Single file
rustor src/Controller.php

# Directory (recursive)
rustor src/

# Multiple paths
rustor src/ app/ tests/

# Glob patterns (shell expansion)
rustor src/**/*.php
```

## Options

### Mode Options

#### `--check`

Check for issues without applying fixes. This is the default mode.

```bash
rustor src/ --check
```

#### `--fix`

Apply fixes to files. Creates backups by default.

```bash
rustor src/ --fix
```

#### `-n, --dry-run`

Alias for `--check`. Show changes without applying them.

```bash
rustor src/ --dry-run
```

#### `--fixer`

Run formatting fixers only (no refactoring rules). See [Fixers Reference](fixers.md).

```bash
# Check formatting issues
rustor src/ --fixer

# Fix formatting issues
rustor src/ --fixer --fix

# Run specific fixer
rustor src/ --fixer --rule no_trailing_whitespace
```

#### `--fixer-config <PATH>`

Load a PHP-CS-Fixer configuration file (`.php-cs-fixer.php` or `.php-cs-fixer.dist.php`).

```bash
# Use project's PHP-CS-Fixer config
rustor src/ --fixer --fixer-config .php-cs-fixer.php

# Use dist config
rustor src/ --fixer --fixer-config .php-cs-fixer.dist.php
```

Rustor parses the PHP config file and extracts:
- Rules and their options (`->setRules([...])`)
- Line ending preference (`->setLineEnding("\n")`)
- Indentation style (`->setIndent("    ")`)
- Risky rules allowance (`->setRiskyAllowed(true)`)
- Finder configuration (`->in()`, `->exclude()`, `->notName()`, `->notPath()`)
- Preset expansion (`@PSR12`, `@Symfony`, `@PhpCsFixer`)

See [Fixers Reference](fixers.md#using-php-cs-fixer-config-files) for full details.

#### `--fixer-preset <PRESET>`

Use a fixer preset. Available presets:

| Preset | Description |
|--------|-------------|
| `psr12` | PSR-12 coding standard (default) |
| `symfony` | Symfony coding standard (extends PSR-12) |
| `phpcsfixer` | PHP-CS-Fixer preset (extends Symfony) |

```bash
rustor src/ --fixer --fixer-preset symfony
```

### Rule Selection

#### `-r, --rule <RULE>`

Specify rules to run. Can be used multiple times. Overrides config file settings.

```bash
rustor src/ --rule is_null --rule array_push
```

#### `--preset <PRESET>`

Use a preset rule configuration.

| Preset | Description |
|--------|-------------|
| `recommended` | Safe, widely-applicable rules (default) |
| `performance` | Performance-focused optimizations |
| `modernize` | Syntax modernization for newer PHP |
| `all` | All available rules |

```bash
rustor src/ --preset modernize
```

#### `--php-version <VERSION>`

Target PHP version. Only rules compatible with this version will run.

Valid versions: `5.4`, `5.5`, `5.6`, `7.0`, `7.1`, `7.2`, `7.3`, `7.4`, `8.0`, `8.1`, `8.2`, `8.3`, `8.4`

```bash
rustor src/ --php-version 7.4
```

#### `--category <CATEGORY>`

Only run rules in a specific category.

| Category | Description |
|----------|-------------|
| `performance` | Rules that improve runtime performance |
| `modernization` | Rules that modernize syntax |
| `simplification` | Rules that simplify code |
| `compatibility` | Rules that ensure compatibility |

```bash
rustor src/ --category modernization
```

#### `--list-rules`

List all available rules with descriptions and exit.

```bash
rustor --list-rules
```

### Output Options

#### `--format <FORMAT>`

Output format. Default: `text`

| Format | Description | Use Case |
|--------|-------------|----------|
| `text` | Human-readable summary | Terminal output |
| `diff` | Colored unified diff | Code review |
| `json` | Machine-readable JSON | Scripting, automation |
| `sarif` | Static Analysis Results Format | GitHub Code Scanning |
| `html` | Standalone HTML report | Documentation |
| `checkstyle` | Checkstyle XML | Jenkins, CI tools |
| `github` | GitHub Actions annotations | Pull request checks |

```bash
rustor src/ --format diff
rustor src/ --format sarif > results.sarif
rustor src/ --format github
```

#### `--json`

Shorthand for `--format json`.

```bash
rustor src/ --json
```

#### `-v, --verbose`

Show verbose output including file paths being processed.

```bash
rustor src/ --verbose
```

#### `--no-progress`

Disable progress bar output. Automatically disabled for non-TTY output.

```bash
rustor src/ --no-progress
```

### Configuration

#### `--config <PATH>`

Path to configuration file. Default: auto-detect `.rustor.toml` in current or parent directories.

```bash
rustor src/ --config /path/to/.rustor.toml
```

#### `--no-config`

Ignore configuration files.

```bash
rustor src/ --no-config
```

### Git Integration

#### `--staged`

Only check git-staged files. Useful for pre-commit hooks.

```bash
rustor --staged
rustor --staged --fix
```

#### `--since <REF>`

Only check files changed since a git reference (branch, tag, or commit).

```bash
# Files changed since main branch
rustor --since origin/main

# Files changed in last 5 commits
rustor --since HEAD~5

# Files changed since a tag
rustor --since v1.0.0
```

### Baseline Support

#### `--generate-baseline`

Generate a baseline file to stdout. Captures current issues for gradual adoption.

```bash
rustor src/ --generate-baseline > .rustor-baseline.json
```

#### `--baseline <FILE>`

Use a baseline file to filter results. Only shows new issues not in the baseline.

```bash
rustor src/ --baseline .rustor-baseline.json
```

### Caching

#### `--no-cache`

Disable caching. Always re-process all files.

```bash
rustor src/ --no-cache
```

#### `--clear-cache`

Clear the cache before running.

```bash
rustor src/ --clear-cache
```

### Fix Safety

#### `--backup`

Create backup files before applying fixes. Default: `true`.

```bash
rustor src/ --fix --backup
```

#### `--no-backup`

Disable backup creation when fixing files.

```bash
rustor src/ --fix --no-backup
```

#### `--backup-dir <DIR>`

Directory to store backup files. Default: `.rustor-backups`

```bash
rustor src/ --fix --backup-dir /tmp/backups
```

#### `--verify`

Verify fixed files parse correctly. Restores from backup on parse failure.

```bash
rustor src/ --fix --verify
```

### Watch Mode

#### `-w, --watch`

Watch mode: re-run analysis when files change.

```bash
rustor src/ --watch
```

### LSP Server

#### `--lsp`

Run as Language Server Protocol server for IDE integration.

```bash
rustor --lsp
```

See [IDE Integration](lsp.md) for configuration.

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success (no issues found or all fixes applied) |
| 1 | Issues found (in check mode) or error occurred |

## Examples

### Basic Usage

```bash
# Check a directory
rustor src/

# Check with verbose output
rustor src/ -v

# Show diff of proposed changes
rustor src/ --format diff

# Apply all fixes
rustor src/ --fix
```

### Rule Selection

```bash
# Use modernize preset
rustor src/ --preset modernize

# Run specific rules only
rustor src/ --rule is_null --rule array_push

# Target PHP 7.4 compatibility
rustor src/ --php-version 7.4

# Only performance rules
rustor src/ --category performance
```

### CI/CD Integration

```bash
# GitHub Actions annotations
rustor src/ --format github

# SARIF for code scanning
rustor src/ --format sarif > results.sarif

# Checkstyle for Jenkins
rustor src/ --format checkstyle > checkstyle.xml

# Exit with error if issues found
rustor src/ && echo "No issues" || echo "Issues found"
```

### Git Workflows

```bash
# Pre-commit hook
rustor --staged --fix

# Check only changed files in PR
rustor --since origin/main

# Check changes since last release
rustor --since v2.0.0
```

### Gradual Adoption

```bash
# Generate baseline
rustor src/ --generate-baseline > .rustor-baseline.json

# Only report new issues
rustor src/ --baseline .rustor-baseline.json

# Fix only new issues
rustor src/ --baseline .rustor-baseline.json --fix
```

### Safe Fixing

```bash
# Fix with backup (default)
rustor src/ --fix

# Fix with custom backup location
rustor src/ --fix --backup-dir ~/.rustor-backups

# Fix and verify results
rustor src/ --fix --verify

# Fix without backup (careful!)
rustor src/ --fix --no-backup
```

### Development

```bash
# Watch mode during development
rustor src/ --watch

# Clear cache and recheck
rustor src/ --clear-cache

# Skip cache entirely
rustor src/ --no-cache
```

## Environment Variables

Rustor respects the following environment variables:

| Variable | Description |
|----------|-------------|
| `NO_COLOR` | Disable colored output |
| `CLICOLOR_FORCE` | Force colored output |

## Subcommands

### `analyze` - PHPStan-compatible Static Analysis

Run PHPStan-compatible static analysis on PHP files. This command provides comprehensive static analysis with full compatibility for PHPStan configuration files and error formats.

```bash
rustor analyze [OPTIONS] [PATHS]...
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --configuration <FILE>` | PHPStan config file (phpstan.neon) |
| `-l, --level <LEVEL>` | Analysis level (0-9, max) |
| `--error-format <FORMAT>` | Output format: raw, json, table, github |
| `--generate-baseline <FILE>` | Generate baseline file |
| `--baseline <FILE>` | Use baseline file to filter issues |
| `--phpstan-compat` | PHPStan exact compatibility mode |
| `-v, --verbose` | Verbose output |
| `-h, --help` | Print help |

#### PHPStan Compatibility Mode

By default, rustor's analyzer has some lenient behaviors that differ from PHPStan:

- Classes with `__get` magic method: rustor issues **warnings** for undefined property access (since `__get` handles it), while PHPStan issues **errors**

Use `--phpstan-compat` to enable strict PHPStan compatibility mode, which ensures rustor produces identical results to PHPStan:

```bash
# Default mode (lenient)
rustor analyze src/ --level 1

# PHPStan exact compatibility mode
rustor analyze src/ --level 1 --phpstan-compat
```

#### PHPStan Compatibility Status

| Level | Match Rate | Status |
|-------|------------|--------|
| 0 | 100% | Full compatibility |
| 1 | 100% | Full compatibility |
| 2 | 100% | Full compatibility |
| 3 | 100% | Full compatibility |
| 4 | 92% | Missing: comparison always true/false |
| 5 | 92% | Missing: comparison always true/false |
| 6 | 85% | Missing: comparison always true/false, iterable value type |

#### Analysis Levels

Rustor supports PHPStan analysis levels 0-6:

| Level | Description | Checks |
|-------|-------------|--------|
| 0 | Basic checks | Undefined functions, classes, static methods, class constants, argument counts, missing returns |
| 1 | Variable analysis | Undefined variables, possibly undefined variables, unused constructor parameters |
| 2 | Type-aware checks | Unknown methods/properties on known types, method argument counts |
| 3 | Return types | Return type validation, property type validation |
| 4 | Dead code | Unreachable statements, always-false instanceof, redundant type narrowing, unused function results |
| 5 | Argument types | Argument type mismatches in function/method calls |
| 6 | Missing typehints | Parameters, return types, and properties without type declarations |

#### PHPStan Error Identifiers

Rustor uses PHPStan-compatible error identifiers:

| Identifier | Level | Description |
|------------|-------|-------------|
| `function.notFound` | 0 | Undefined function call |
| `class.notFound` | 0 | Undefined class |
| `staticMethod.notFound` | 0 | Undefined static method |
| `classConstant.notFound` | 0 | Undefined class constant |
| `arguments.count` | 0 | Wrong argument count |
| `return.missing` | 0 | Missing return statement |
| `variable.undefined` | 1 | Undefined variable |
| `variable.possiblyUndefined` | 1 | Possibly undefined variable |
| `constructor.unusedParameter` | 1 | Unused constructor parameter |
| `method.notFound` | 2 | Undefined method on known type |
| `property.notFound` | 2 | Undefined property on known type |
| `return.type` | 3 | Return type mismatch |
| `property.type` | 3 | Property type mismatch |
| `return.void` | 3 | Void function returns value |
| `deadCode.unreachable` | 4 | Unreachable code |
| `instanceof.alwaysFalse` | 4 | Always-false instanceof |
| `function.alreadyNarrowedType` | 4 | Redundant type narrowing |
| `function.resultUnused` | 4 | Unused pure function result |
| `argument.type` | 5 | Argument type mismatch |
| `missingType.parameter` | 6 | Missing parameter type |
| `missingType.return` | 6 | Missing return type |
| `missingType.property` | 6 | Missing property type |

#### Configuration File Support

Rustor fully supports PHPStan NEON configuration files:

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
    phpVersion: 80100
    treatPhpDocTypesAsCertain: true
    ignoreErrors:
        - '#Call to undefined function legacy_func#'
        -
            message: '#Variable \$data might not be defined#'
            path: src/Legacy/*.php

includes:
    - phpstan-baseline.neon
```

See [Static Analysis](analyze.md) for comprehensive configuration documentation.

#### Examples

```bash
# Basic analysis at level 2
rustor analyze src/ --level 2

# Use PHPStan config file
rustor analyze -c phpstan.neon

# Generate baseline for gradual adoption
rustor analyze src/ --level 3 --generate-baseline baseline.neon

# Filter results using baseline
rustor analyze src/ --level 3 --baseline baseline.neon

# JSON output for CI
rustor analyze src/ --level 3 --error-format json

# GitHub Actions annotations
rustor analyze src/ --level 3 --error-format github

# PHPStan exact compatibility mode
rustor analyze src/ --level 3 --phpstan-compat
```

## See Also

- [Static Analysis](analyze.md) - Comprehensive analysis documentation
- [Rules Reference](rules.md) - Complete list of all refactoring rules
- [Configuration](configuration.md) - `.rustor.toml` file format
- [IDE Integration](lsp.md) - LSP server setup
