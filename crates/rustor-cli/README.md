# rustor-cli

Command-line interface for rustor PHP refactoring tool.

## Installation

```bash
cargo build --release
# Binary at: target/release/rustor
```

## Quick Start

```bash
# Check for issues
rustor src/

# Show diff of changes
rustor src/ --format diff

# Apply fixes
rustor src/ --fix

# List available rules
rustor --list-rules
```

## Features

### Output Formats

| Format | Flag | Description |
|--------|------|-------------|
| Text | `--format text` | Human-readable summary (default) |
| Diff | `--format diff` | Colored unified diff |
| JSON | `--format json` | Machine-readable |
| SARIF | `--format sarif` | Static Analysis Results Format |
| HTML | `--format html` | Standalone report |
| Checkstyle | `--format checkstyle` | XML for CI tools |
| GitHub | `--format github` | Actions annotations |

### Git Integration

```bash
# Pre-commit hook
rustor --staged --fix

# Check changed files
rustor --since origin/main
```

### Baseline Support

```bash
# Generate baseline
rustor src/ --generate-baseline > .rustor-baseline.json

# Only new issues
rustor src/ --baseline .rustor-baseline.json
```

### Configuration

Create `.rustor.toml`:

```toml
[php]
version = "8.0"

[rules]
preset = "recommended"
enabled = ["string_contains"]
disabled = ["sizeof"]

[paths]
exclude = ["vendor/"]
```

### Watch Mode

```bash
rustor src/ --watch
```

### LSP Server

```bash
rustor --lsp
```

See [IDE Integration](../../docs/lsp.md) for editor setup.

### Safe Fixing

```bash
# With backup (default)
rustor src/ --fix

# With verification
rustor src/ --fix --verify

# Custom backup location
rustor src/ --fix --backup-dir /tmp/backups
```

## Modules

| Module | Description |
|--------|-------------|
| `main.rs` | CLI entry point, argument parsing |
| `process.rs` | File processing, rule execution |
| `output.rs` | Output formatting |
| `config.rs` | Configuration file parsing |
| `cache.rs` | File caching for performance |
| `watch.rs` | Watch mode with file system notifications |
| `git.rs` | Git integration (`--staged`, `--since`) |
| `baseline.rs` | Baseline generation and filtering |
| `ignore.rs` | Inline ignore comment parsing |
| `backup.rs` | Backup and restore functionality |
| `lsp.rs` | Language Server Protocol server |

## Performance

- **Parallel processing** - Uses rayon for multi-core execution
- **Intelligent caching** - Skips unchanged files
- **Incremental checking** - `--staged` and `--since` for CI

Benchmark (Laravel, 2,752 files):
```
Time: ~1.2 seconds
```

## See Also

- [CLI Reference](../../docs/cli.md) - Complete option reference
- [Configuration](../../docs/configuration.md) - Config file format
- [Development Guide](../../docs/development.md) - Architecture overview
