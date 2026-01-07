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

## See Also

- [Rules Reference](rules.md) - Complete list of all rules
- [Configuration](configuration.md) - `.rustor.toml` file format
- [IDE Integration](lsp.md) - LSP server setup
