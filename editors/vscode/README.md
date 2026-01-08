# Rustor - PHP Refactoring for VS Code

Automated PHP code modernization and refactoring powered by the Rustor tool.

## Features

- **Real-time Diagnostics**: See refactoring suggestions as you type
- **Quick Fixes**: Apply transformations with a single click
- **Workspace Scanning**: Fix entire projects at once
- **25+ Rules**: Comprehensive PHP modernization rules

## Installation

### Prerequisites

1. Install the `rustor` binary:
   ```bash
   cargo install rustor
   ```
   Or download from [releases](https://github.com/rustor/rustor/releases).

2. Ensure `rustor` is in your PATH, or configure the path in settings.

### From VS Code

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "Rustor"
4. Click Install

### From VSIX

1. Download the `.vsix` file from releases
2. In VS Code: Extensions → ... → Install from VSIX

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `rustor.path` | `"rustor"` | Path to the rustor executable |
| `rustor.enable` | `true` | Enable/disable the language server |
| `rustor.phpVersion` | `"8.2"` | Target PHP version for rules |
| `rustor.preset` | `"recommended"` | Rule preset (recommended, performance, modernize, all) |
| `rustor.trace.server` | `"off"` | Trace communication with the server |

## Commands

- **Rustor: Restart Language Server** - Restart the LSP server
- **Rustor: Fix Current File** - Apply all fixes to the current file
- **Rustor: Fix Entire Workspace** - Apply fixes to all PHP files in workspace

## Example Transformations

### Before
```php
if (is_null($value)) {
    $result = isset($x) ? $x : 'default';
    array_push($arr, $item);
}
```

### After
```php
if ($value === null) {
    $result = $x ?? 'default';
    $arr[] = $item;
}
```

## Available Rules

### Recommended Preset
- `array_push` - `array_push($arr, $val)` → `$arr[] = $val`
- `array_syntax` - `array()` → `[]`
- `is_null` - `is_null($x)` → `$x === null`
- `isset_coalesce` - `isset($x) ? $x : $default` → `$x ?? $default`
- `sizeof` - `sizeof($arr)` → `count($arr)`

### Modernize Preset (PHP 7.4+)
- `arrow_functions` - Closure to arrow function
- `match_expression` - Switch to match (PHP 8.0+)
- `null_safe_operator` - Null-safe method calls (PHP 8.0+)
- `constructor_promotion` - Constructor property promotion (PHP 8.0+)
- `readonly_properties` - Add readonly modifier (PHP 8.1+)
- `override_attribute` - Add #[Override] attribute (PHP 8.3+)
- And more...

## Troubleshooting

### Language Server Not Starting

1. Check that `rustor` is installed: `rustor --version`
2. Verify the path in settings: `rustor.path`
3. Check the Output panel for errors (View → Output → Rustor)

### No Diagnostics Appearing

1. Ensure the file is a `.php` file
2. Check that `rustor.enable` is `true`
3. Try restarting the language server

## Building from Source

```bash
cd editors/vscode
npm install
npm run compile
npm run package  # Creates .vsix file
```

## License

MIT License - See [LICENSE](LICENSE) for details.
