# Rustor

A blazing-fast PHP refactoring tool written in Rust.

Rustor automatically modernizes your PHP codebase by applying safe, semantic transformations. It parses PHP code into an AST, applies configurable refactoring rules, and outputs clean, format-preserving edits.

## Features

- **23 refactoring rules** covering modernization, performance, and compatibility
- **Blazing fast** - processes thousands of files in seconds using parallel execution
- **Format-preserving** - maintains your code style while making targeted changes
- **Safe by default** - dry-run mode, backup support, and parse verification
- **IDE integration** - built-in LSP server for real-time diagnostics
- **CI/CD ready** - SARIF, Checkstyle, and GitHub Actions output formats
- **Configurable** - `.rustor.toml` configuration with presets and per-rule options

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/your-org/rustor.git
cd rustor
cargo build --release

# Add to PATH
export PATH="$PATH:$(pwd)/target/release"
```

### Basic Usage

```bash
# Check for issues (dry-run mode)
rustor src/

# Show detailed diff of proposed changes
rustor src/ --format diff

# Apply fixes
rustor src/ --fix

# Use a specific preset
rustor src/ --preset modernize

# Target a specific PHP version
rustor src/ --php-version 8.0
```

### Example Transformations

```php
// Before
array_push($items, $value);
$x = isset($data['key']) ? $data['key'] : 'default';
if (is_null($result)) { }
$len = sizeof($array);
$power = pow($base, $exp);

// After
$items[] = $value;
$x = $data['key'] ?? 'default';
if ($result === null) { }
$len = count($array);
$power = $base ** $exp;
```

## Documentation

- **[Rules Reference](docs/rules.md)** - Complete list of all 23 refactoring rules
- **[CLI Reference](docs/cli.md)** - All command-line options and flags
- **[Configuration](docs/configuration.md)** - `.rustor.toml` file format
- **[IDE Integration](docs/lsp.md)** - LSP server setup for VS Code, Neovim, etc.
- **[Development Guide](docs/development.md)** - Architecture and contributing

## Rule Presets

| Preset | Description | Rules |
|--------|-------------|-------|
| `recommended` | Safe, widely-applicable rules (default) | 6 rules |
| `performance` | Performance-focused optimizations | 5 rules |
| `modernize` | Syntax modernization for newer PHP | 13 rules |
| `all` | All available rules | 23 rules |

## Output Formats

| Format | Description | Use Case |
|--------|-------------|----------|
| `text` | Human-readable summary (default) | Terminal output |
| `diff` | Colored unified diff | Code review |
| `json` | Machine-readable JSON | Scripting |
| `sarif` | Static Analysis Results Format | GitHub Code Scanning |
| `html` | Standalone HTML report | Documentation |
| `checkstyle` | Checkstyle XML | Jenkins, CI tools |
| `github` | GitHub Actions annotations | Pull request checks |

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run rustor
  run: |
    rustor src/ --format github

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v2
  with:
    sarif_file: rustor-results.sarif
```

### Pre-commit Hook

```bash
#!/bin/bash
rustor --staged --fix
```

### Baseline for Gradual Adoption

```bash
# Generate baseline of existing issues
rustor src/ --generate-baseline > .rustor-baseline.json

# Only report new issues
rustor src/ --baseline .rustor-baseline.json
```

## IDE Integration

Rustor includes a built-in LSP server for real-time diagnostics:

```bash
rustor --lsp
```

See [IDE Integration](docs/lsp.md) for setup instructions for VS Code, Neovim, and other editors.

## Configuration

Create a `.rustor.toml` file in your project root:

```toml
[php]
version = "8.0"

[rules]
preset = "recommended"
enabled = ["string_contains", "null_safe_operator"]
disabled = ["sizeof"]

[paths]
include = ["src/", "app/"]
exclude = ["vendor/", "tests/fixtures/"]

[fix]
backup = true
backup_dir = ".rustor-backups"
```

See [Configuration](docs/configuration.md) for full details.

## Performance

Rustor is designed for speed:

- **Parallel processing** using Rayon for multi-core utilization
- **Intelligent caching** to skip unchanged files
- **Incremental checking** with `--staged` and `--since` for CI

Benchmark on Laravel framework (2,752 PHP files):
```
Files processed: 2,752
Time: ~1.2 seconds
```

## Requirements

- Rust 1.70+ (for building)
- PHP files with valid syntax

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

See [Development Guide](docs/development.md) for architecture overview and contribution guidelines.
