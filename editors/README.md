# Editor Integrations

Rustor includes a Language Server Protocol (LSP) implementation for IDE integration.

## Starting the LSP Server

```bash
rustor --lsp
```

Optional flags:
- `--php-version <VERSION>` - Target PHP version (default: 8.2)
- `--preset <PRESET>` - Rule preset (recommended, performance, modernize, all)

## VS Code

See [vscode/README.md](vscode/README.md) for the full extension.

### Quick Setup

1. Install the extension from the VS Code marketplace
2. Ensure `rustor` is in your PATH
3. Open a PHP file

### Building from Source

```bash
cd editors/vscode
npm install
npm run compile
npm run package
```

## Neovim

### Using nvim-lspconfig

Add to your Lua config:

```lua
-- Register rustor as an LSP server
require('lspconfig.configs').rustor = {
  default_config = {
    cmd = { 'rustor', '--lsp' },
    filetypes = { 'php' },
    root_dir = function(fname)
      return vim.fn.getcwd()
    end,
  },
}

-- Enable rustor
require('lspconfig').rustor.setup{}
```

### Manual Setup

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "php",
  callback = function()
    vim.lsp.start({
      name = "rustor",
      cmd = { "rustor", "--lsp" },
      root_dir = vim.fn.getcwd(),
    })
  end,
})
```

## Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "php"
language-servers = ["rustor"]

[language-server.rustor]
command = "rustor"
args = ["--lsp"]
```

## Zed

Add to settings:

```json
{
  "lsp": {
    "rustor": {
      "binary": {
        "path": "rustor",
        "arguments": ["--lsp"]
      }
    }
  },
  "languages": {
    "PHP": {
      "language_servers": ["rustor"]
    }
  }
}
```

## Sublime Text

Install [LSP](https://packagecontrol.io/packages/LSP) package, then add to LSP settings:

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

## Emacs (lsp-mode)

Add to your config:

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(php-mode . "php"))

  (lsp-register-client
    (make-lsp-client
      :new-connection (lsp-stdio-connection '("rustor" "--lsp"))
      :major-modes '(php-mode)
      :server-id 'rustor)))
```

## LSP Features

Rustor's LSP server provides:

- **Diagnostics**: Real-time suggestions as you edit
- **Code Actions**: Quick fixes for all 25+ rules
- **Document Formatting**: (planned)

## Troubleshooting

### Server Not Starting

1. Verify rustor is installed: `rustor --version`
2. Check the server can start: `rustor --lsp`
3. Look at editor logs for connection errors

### No Diagnostics

1. Ensure the file is saved (some editors only analyze saved files)
2. Check if rules apply to your PHP version
3. Verify the preset includes relevant rules

### Performance

For large projects, consider:
- Using `.rustor.toml` to exclude `vendor/`
- Running with specific rules instead of "all"
- Using file caching (enabled by default)
