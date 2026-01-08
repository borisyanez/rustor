# Configuration

Rustor can be configured using a `.rustor.toml` file in your project root.

## File Location

Rustor looks for configuration files in the following order:

1. Path specified with `--config` flag
2. `.rustor.toml` in current directory
3. `.rustor.toml` in parent directories (up to filesystem root)

Use `--no-config` to ignore configuration files.

## Complete Example

```toml
# .rustor.toml - Rustor configuration file

[php]
# Target PHP version - only rules compatible with this version will run
version = "8.0"

[rules]
# Use a preset as base configuration
preset = "recommended"

# Enable additional rules on top of preset
enabled = [
    "string_contains",
    "null_safe_operator",
    "match_expression"
]

# Disable specific rules from preset
disabled = [
    "sizeof"
]

[paths]
# Only process these paths (relative to config file)
include = [
    "src/",
    "app/",
    "lib/"
]

# Exclude these patterns
exclude = [
    "vendor/",
    "node_modules/",
    "tests/fixtures/",
    "**/*.generated.php"
]

[output]
# Output format: text, json, diff, sarif, html, checkstyle, github
format = "text"

# Enable/disable colors (auto-detected by default)
color = true

[fix]
# Create backups before applying fixes
backup = true

# Directory for backup files
backup_dir = ".rustor-backups"

# Verify fixed files parse correctly
verify = true

[cache]
# Enable caching (default: true)
enabled = true

# Cache directory (default: .rustor-cache)
dir = ".rustor-cache"

[skip]
# Skip specific rules for specific paths (Rector-style)
rename_class = ["src/Legacy/*", "tests/fixtures/*"]
sizeof = ["vendor/*"]
# Skip ALL rules for certain paths
"*" = ["generated/*", "*.generated.php"]

# Rule-specific configuration
[rules.string_contains]
# Also convert loose comparisons (== and !=)
loose_comparison = false
```

## Configuration Sections

### `[php]`

PHP-related settings.

#### `version`

Target PHP version. Only rules compatible with this version will run.

```toml
[php]
version = "8.0"
```

Valid versions: `5.4`, `5.5`, `5.6`, `7.0`, `7.1`, `7.2`, `7.3`, `7.4`, `8.0`, `8.1`, `8.2`, `8.3`, `8.4`

---

### `[rules]`

Rule selection and configuration.

#### `preset`

Base preset to use. Rules from the preset can be modified with `enabled` and `disabled`.

```toml
[rules]
preset = "recommended"
```

| Preset | Description |
|--------|-------------|
| `recommended` | Safe, widely-applicable rules (default) |
| `performance` | Performance-focused optimizations |
| `modernize` | Syntax modernization for newer PHP |
| `all` | All available rules |

#### `enabled`

Additional rules to enable on top of the preset.

```toml
[rules]
preset = "recommended"
enabled = ["string_contains", "null_safe_operator"]
```

#### `disabled`

Rules to disable from the preset.

```toml
[rules]
preset = "all"
disabled = ["sizeof", "join_to_implode"]
```

#### Rule-specific configuration

Some rules support configuration options via `[rules.<rule_name>]`:

```toml
[rules.string_contains]
loose_comparison = true
```

---

### `[paths]`

File and directory filtering.

#### `include`

Only process files in these paths. Paths are relative to the config file.

```toml
[paths]
include = ["src/", "app/"]
```

#### `exclude`

Exclude these patterns. Supports glob patterns.

```toml
[paths]
exclude = [
    "vendor/",
    "node_modules/",
    "**/*.generated.php",
    "**/fixtures/**"
]
```

---

### `[skip]`

Skip specific rules for specific paths. This is similar to Rector's `withSkip()` configuration.

#### Rule-specific skipping

Skip a rule only for certain paths:

```toml
[skip]
rename_class = ["src/Legacy/*", "tests/fixtures/*"]
sizeof = ["vendor/*"]
```

In this example:
- `rename_class` rule will be skipped for files in `src/Legacy/` and `tests/fixtures/`
- `sizeof` rule will be skipped for files in `vendor/`

#### Wildcard skipping

Skip ALL rules for certain paths using the `"*"` key:

```toml
[skip]
"*" = ["generated/*", "*.generated.php"]
```

This skips all rules for:
- All files in the `generated/` directory
- All files ending with `.generated.php`

#### Glob patterns

Supports standard glob patterns:

```toml
[skip]
rename_class = [
    "src/Legacy/*",           # All files in Legacy directory
    "**/fixtures/**",         # Any fixtures directory
    "**/*.test.php",          # All test files
    "tests/**"                # Everything under tests
]
```

---

### `[output]`

Output formatting options.

#### `format`

Default output format.

```toml
[output]
format = "diff"
```

| Format | Description |
|--------|-------------|
| `text` | Human-readable summary (default) |
| `diff` | Colored unified diff |
| `json` | Machine-readable JSON |
| `sarif` | Static Analysis Results Format |
| `html` | Standalone HTML report |
| `checkstyle` | Checkstyle XML |
| `github` | GitHub Actions annotations |

#### `color`

Enable or disable colored output. Default: auto-detect based on terminal.

```toml
[output]
color = true
```

---

### `[fix]`

Options for fix mode.

#### `backup`

Create backup files before applying fixes. Default: `true`.

```toml
[fix]
backup = true
```

#### `backup_dir`

Directory for backup files. Default: `.rustor-backups`.

```toml
[fix]
backup_dir = ".rustor-backups"
```

#### `verify`

Verify fixed files parse correctly. Restores from backup on failure. Default: `false`.

```toml
[fix]
verify = true
```

---

### `[cache]`

Caching options.

#### `enabled`

Enable or disable caching. Default: `true`.

```toml
[cache]
enabled = true
```

#### `dir`

Cache directory. Default: `.rustor-cache`.

```toml
[cache]
dir = ".rustor-cache"
```

---

## Per-Rule Configuration

### `string_contains`

```toml
[rules.string_contains]
# Also convert loose comparisons (strpos($s, $n) == false)
# Default: false (only strict === and !==)
loose_comparison = false
```

---

## Configuration Precedence

Configuration is merged in the following order (later values override earlier):

1. Default values
2. `.rustor.toml` configuration file
3. Command-line arguments

For example:

```bash
# Config file sets preset = "recommended"
# CLI overrides to use only is_null rule
rustor src/ --rule is_null
```

---

## Minimal Configurations

### Laravel Project

```toml
[php]
version = "8.1"

[rules]
preset = "modernize"

[paths]
include = ["app/", "routes/", "database/"]
exclude = ["vendor/"]
```

### Legacy Project (PHP 7.4)

```toml
[php]
version = "7.4"

[rules]
preset = "recommended"
enabled = ["assign_coalesce"]

[paths]
exclude = ["vendor/", "legacy/"]
```

### CI Configuration

```toml
[rules]
preset = "all"

[output]
format = "sarif"
color = false

[fix]
backup = false
```

### Strict Configuration

```toml
[php]
version = "8.2"

[rules]
preset = "all"

[fix]
backup = true
backup_dir = "/tmp/rustor-backups"
verify = true
```

---

## Inline Configuration

In addition to the config file, you can use inline comments to disable rules:

```php
<?php

// rustor-ignore-file: sizeof
// Disables sizeof rule for entire file

class Example
{
    public function test()
    {
        // rustor-ignore
        if (is_null($value)) { }  // This line ignored

        // rustor-ignore: is_null, array_push
        $x = is_null($y);  // Specific rules ignored

        $count = sizeof($arr);  // rustor-ignore-line
    }
}
```

### Comment Formats

| Format | Scope |
|--------|-------|
| `// rustor-ignore-file` | Entire file |
| `// rustor-ignore-file: rule1, rule2` | Entire file, specific rules |
| `// rustor-ignore` | Next line |
| `// rustor-ignore: rule1, rule2` | Next line, specific rules |
| `// rustor-ignore-line` | Same line |
| `/* rustor-ignore */` | Block comment style |

---

## Validating Configuration

Check if your configuration is valid:

```bash
# List rules that would run with current config
rustor src/ --list-rules

# Dry-run to see configuration in action
rustor src/ --verbose
```

---

## See Also

- [CLI Reference](cli.md) - Command-line options
- [Rules Reference](rules.md) - All available rules
