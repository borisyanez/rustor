# IDE Integration

Rustor includes a built-in Language Server Protocol (LSP) server for real-time diagnostics and code actions in your IDE.

## Features

- **Real-time diagnostics** - See rustor suggestions as you type
- **Quick fixes** - Apply fixes with one click or keyboard shortcut
- **Parse error reporting** - Immediate feedback on syntax errors
- **Zero configuration** - Works out of the box with any LSP-compatible editor

## Starting the LSP Server

```bash
rustor --lsp
```

The server communicates via stdin/stdout using the LSP protocol.

---

## Editor Setup

### Visual Studio Code

#### Option 1: Manual Configuration

Add to your `settings.json`:

```json
{
    "languageServerExample.serverPath": "/path/to/rustor",
    "languageServerExample.serverArgs": ["--lsp"]
}
```

#### Option 2: Create a VS Code Extension

Create a simple extension with `package.json`:

```json
{
    "name": "rustor",
    "displayName": "Rustor PHP",
    "description": "PHP refactoring with rustor",
    "version": "0.1.0",
    "engines": {
        "vscode": "^1.75.0"
    },
    "categories": ["Linters", "Programming Languages"],
    "activationEvents": ["onLanguage:php"],
    "main": "./out/extension.js",
    "contributes": {
        "configuration": {
            "type": "object",
            "title": "Rustor",
            "properties": {
                "rustor.path": {
                    "type": "string",
                    "default": "rustor",
                    "description": "Path to rustor executable"
                },
                "rustor.enable": {
                    "type": "boolean",
                    "default": true,
                    "description": "Enable rustor diagnostics"
                }
            }
        }
    }
}
```

And `extension.ts`:

```typescript
import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('rustor');
    const serverPath = config.get<string>('path', 'rustor');

    const serverOptions: ServerOptions = {
        run: { command: serverPath, args: ['--lsp'] },
        debug: { command: serverPath, args: ['--lsp'] }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'php' }]
    };

    client = new LanguageClient(
        'rustor',
        'Rustor PHP',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
}
```

---

### Neovim

#### Using nvim-lspconfig

Add to your Neovim configuration:

```lua
-- ~/.config/nvim/lua/plugins/lsp.lua

local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define rustor as a custom LSP server
if not configs.rustor then
    configs.rustor = {
        default_config = {
            cmd = { 'rustor', '--lsp' },
            filetypes = { 'php' },
            root_dir = lspconfig.util.root_pattern('.rustor.toml', 'composer.json', '.git'),
            settings = {},
        },
    }
end

-- Enable rustor
lspconfig.rustor.setup({
    on_attach = function(client, bufnr)
        -- Enable code actions
        vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, { buffer = bufnr })
    end,
})
```

#### Using lazy.nvim

```lua
{
    'neovim/nvim-lspconfig',
    config = function()
        local lspconfig = require('lspconfig')
        local configs = require('lspconfig.configs')

        configs.rustor = {
            default_config = {
                cmd = { 'rustor', '--lsp' },
                filetypes = { 'php' },
                root_dir = lspconfig.util.root_pattern('.rustor.toml', 'composer.json', '.git'),
            },
        }

        lspconfig.rustor.setup({})
    end,
}
```

---

### PhpStorm / IntelliJ IDEA

PhpStorm supports LSP servers through plugins. There are two approaches:

#### Option 1: LSP4IJ Plugin (Recommended)

1. **Install LSP4IJ Plugin**
   - Go to `Settings/Preferences` → `Plugins`
   - Search for "LSP4IJ" or "LSP Support"
   - Install and restart PhpStorm

2. **Configure Rustor LSP Server**
   - Go to `Settings/Preferences` → `Languages & Frameworks` → `Language Server Protocol` → `Server Definitions`
   - Click `+` to add a new server
   - Configure:
     ```
     Name: Rustor
     Extension: php
     Command: /path/to/rustor --lsp
     ```
   - Or use shell script for better path resolution:
     ```
     Command: /bin/bash
     Args: -c "rustor --lsp"
     ```

3. **Enable for PHP Files**
   - Ensure "Enabled" is checked
   - Set "File type" to `PHP` or `*.php`
   - Click "OK" to save

4. **Verify Installation**
   - Open a PHP file
   - You should see rustor diagnostics in the editor
   - Use `Alt+Enter` (Windows/Linux) or `⌥⏎` (macOS) to see quick fixes

#### Option 2: External Tool Integration

If LSP4IJ doesn't work, use rustor as an external tool for on-demand analysis:

1. **Add External Tool**
   - Go to `Settings/Preferences` → `Tools` → `External Tools`
   - Click `+` to add a new tool
   - Configure:
     ```
     Name: Rustor Analyze
     Description: Run rustor analysis
     Program: /path/to/rustor
     Arguments: $FilePath$ --format diff
     Working directory: $ProjectFileDir$
     ```

2. **Add Keyboard Shortcut** (Optional)
   - Go to `Settings/Preferences` → `Keymap`
   - Search for "Rustor Analyze"
   - Add keyboard shortcut (e.g., `Ctrl+Alt+R`)

3. **Usage**
   - Right-click in editor → `External Tools` → `Rustor Analyze`
   - Or use your keyboard shortcut
   - Results appear in the "Run" tool window

#### Option 3: File Watcher for Auto-Analysis

For automatic analysis on save:

1. **Add File Watcher**
   - Go to `Settings/Preferences` → `Tools` → `File Watchers`
   - Click `+` to add a custom watcher
   - Configure:
     ```
     Name: Rustor
     File type: PHP
     Scope: Project Files
     Program: /path/to/rustor
     Arguments: $FilePath$ --format checkstyle
     Output paths to refresh: $FilePath$
     Working directory: $ProjectFileDir$
     ```

2. **Advanced Options**
   - Check "Auto-save edited files to trigger the watcher"
   - Check "Trigger the watcher on external changes"
   - Set "Show console" to "Never" for clean UI

3. **Checkstyle Output**
   - PhpStorm can parse checkstyle format for inline annotations
   - Errors appear in the "Problems" tool window

#### Recommended Setup

For the best PhpStorm experience, combine multiple approaches:

1. **LSP4IJ Plugin** - Real-time diagnostics as you type
2. **External Tool** - On-demand detailed analysis with diff view
3. **File Watcher** - Automatic analysis on save

#### Troubleshooting

**LSP Server Not Starting:**
```bash
# Test rustor path
which rustor
/usr/local/bin/rustor

# Use absolute path in PhpStorm configuration
Command: /usr/local/bin/rustor
Args: --lsp
```

**No Diagnostics Appearing:**
- Check `View` → `Tool Windows` → `LSP Console` for error logs
- Verify rustor works from command line
- Restart PhpStorm after configuration changes

**Performance Issues:**
- Disable other PHP inspections that may conflict
- Use File Watcher only for specific file types
- Increase PhpStorm memory if needed

---

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "php"
language-servers = ["rustor", "intelephense"]

[language-server.rustor]
command = "rustor"
args = ["--lsp"]
```

---

### Sublime Text

Install the LSP package, then add to LSP settings:

```json
{
    "clients": {
        "rustor": {
            "enabled": true,
            "command": ["rustor", "--lsp"],
            "selector": "source.php"
        }
    }
}
```

---

### Emacs (lsp-mode)

Add to your Emacs configuration:

```elisp
(require 'lsp-mode)

(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection '("rustor" "--lsp"))
  :major-modes '(php-mode)
  :server-id 'rustor))

(add-hook 'php-mode-hook #'lsp)
```

---

### Emacs (eglot)

```elisp
(require 'eglot)

(add-to-list 'eglot-server-programs
             '(php-mode . ("rustor" "--lsp")))

(add-hook 'php-mode-hook 'eglot-ensure)
```

---

### Zed

Add to your Zed settings:

```json
{
    "lsp": {
        "rustor": {
            "binary": {
                "path": "/path/to/rustor",
                "arguments": ["--lsp"]
            }
        }
    },
    "languages": {
        "PHP": {
            "language_servers": ["rustor", "intelephense"]
        }
    }
}
```

---

## LSP Capabilities

The rustor LSP server implements:

### Text Document Synchronization

- `textDocument/didOpen` - Analyze file when opened
- `textDocument/didChange` - Re-analyze on changes
- `textDocument/didSave` - Re-analyze on save
- `textDocument/didClose` - Clear diagnostics when closed

### Diagnostics

Rustor publishes diagnostics via `textDocument/publishDiagnostics`:

- **Parse errors** - Severity: Error
- **Refactoring suggestions** - Severity: Hint

Each diagnostic includes:
- Position (line, column)
- Message describing the issue
- Rule name as diagnostic code
- Fix data for code actions

### Code Actions

`textDocument/codeAction` provides quick fixes:

- Each rustor diagnostic has an associated "Fix: ..." code action
- Applying the action replaces the code with the refactored version
- Actions are marked as "preferred" for easy application

---

## Troubleshooting

### Server Not Starting

1. Verify rustor is in your PATH:
   ```bash
   which rustor
   rustor --version
   ```

2. Test the LSP server manually:
   ```bash
   echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{}}}' | rustor --lsp
   ```

3. Check editor logs for errors

### No Diagnostics Appearing

1. Ensure the file is saved with `.php` extension
2. Check if file has valid PHP syntax
3. Verify rustor rules are detecting issues:
   ```bash
   rustor path/to/file.php --format json
   ```

### Performance Issues

For large files, diagnostics may take a moment. The LSP server:
- Processes each file synchronously
- Creates new registry per analysis (could be optimized)

---

## Configuration

The LSP server uses the `recommended` preset by default. Future versions will support:

- Reading `.rustor.toml` configuration
- Dynamic rule configuration
- Workspace-wide settings

---

## Multiple Language Servers

Rustor can run alongside other PHP language servers:

- **Intelephense** - Completion, navigation, hover
- **Phpactor** - Refactoring, completion
- **PHP Language Server** - Basic features

Example with nvim-lspconfig:

```lua
-- Enable both rustor and intelephense
lspconfig.rustor.setup({})
lspconfig.intelephense.setup({})
```

Rustor focuses on refactoring suggestions while other servers handle:
- Code completion
- Go to definition
- Find references
- Hover documentation

---

## See Also

- [CLI Reference](cli.md) - Command-line usage
- [Rules Reference](rules.md) - Available refactoring rules
- [Configuration](configuration.md) - `.rustor.toml` options
