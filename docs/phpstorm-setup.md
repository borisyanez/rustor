# PhpStorm / IntelliJ IDEA Setup Guide

Complete guide to integrating Rustor with PhpStorm and IntelliJ IDEA for PHP development.

---

## Quick Start

**Fastest way to get started:**

1. Install LSP4IJ plugin from JetBrains Marketplace
2. Add Rustor as LSP server with command: `rustor --lsp`
3. Open a PHP file and see real-time diagnostics

---

## Installation Methods

### Method 1: LSP Server (Recommended) ⭐

Get real-time diagnostics and quick fixes as you type.

#### Step 1: Install LSP4IJ Plugin

1. Open PhpStorm
2. Go to **Settings/Preferences** → **Plugins**
3. Click **Marketplace** tab
4. Search for **"LSP4IJ"** or **"LSP Support"**
5. Click **Install**
6. Click **Restart IDE**

#### Step 2: Configure Rustor LSP Server

1. Go to **Settings/Preferences** → **Languages & Frameworks** → **Language Server Protocol** → **Server Definitions**
2. Click **+** (Add) button
3. Fill in the form:

   | Field | Value |
   |-------|-------|
   | **Name** | `Rustor` |
   | **Language** | `PHP` |
   | **Command** | `/usr/local/bin/rustor` (or your rustor path) |
   | **Args** | `--lsp` |

4. Alternative using shell wrapper (if path issues):

   | Field | Value |
   |-------|-------|
   | **Command** | `/bin/bash` |
   | **Args** | `-c "rustor --lsp"` |

5. Click **OK** to save

#### Step 3: Verify Setup

1. Open any PHP file in your project
2. Make a change that triggers a rustor rule (e.g., use `array_push($arr, $x)`)
3. You should see:
   - Yellow/gray underline under the code
   - Hover to see the diagnostic message
   - Press `Alt+Enter` (Windows/Linux) or `⌥⏎` (macOS) for quick fix

**Troubleshooting:**
- Check **View** → **Tool Windows** → **LSP Console** for logs
- Verify rustor path: `which rustor` in terminal
- Restart PhpStorm after configuration

---

### Method 2: External Tool

Run rustor on-demand with keyboard shortcut or menu.

#### Step 1: Add External Tool

1. Go to **Settings/Preferences** → **Tools** → **External Tools**
2. Click **+** (Add) button
3. Configure:

   | Field | Value |
   |-------|-------|
   | **Name** | `Rustor Analyze` |
   | **Description** | `Run rustor static analysis` |
   | **Program** | `/usr/local/bin/rustor` |
   | **Arguments** | `analyze $FilePath$ --level 6 --format diff` |
   | **Working directory** | `$ProjectFileDir$` |

4. Click **OK**

#### Step 2: Add Keyboard Shortcut (Optional)

1. Go to **Settings/Preferences** → **Keymap**
2. Search for **"Rustor Analyze"**
3. Right-click → **Add Keyboard Shortcut**
4. Press your desired key combination (e.g., `Ctrl+Alt+R`)
5. Click **OK**

#### Step 3: Usage

**Via Menu:**
- Right-click in PHP file → **External Tools** → **Rustor Analyze**

**Via Keyboard:**
- Press your keyboard shortcut (e.g., `Ctrl+Alt+R`)

**Results:**
- Output appears in **Run** tool window at bottom
- Shows diff of proposed changes

---

### Method 3: File Watcher

Automatically run rustor when files are saved.

#### Step 1: Add File Watcher

1. Go to **Settings/Preferences** → **Tools** → **File Watchers**
2. Click **+** → **\<custom\>**
3. Configure:

   | Field | Value |
   |-------|-------|
   | **Name** | `Rustor` |
   | **File type** | `PHP` |
   | **Scope** | `Project Files` |
   | **Program** | `/usr/local/bin/rustor` |
   | **Arguments** | `analyze $FilePath$ --level 6 --output checkstyle` |
   | **Output paths to refresh** | `$FilePath$` |
   | **Working directory** | `$ProjectFileDir$` |

4. **Advanced Options:**
   - ✅ Auto-save edited files to trigger the watcher
   - ✅ Trigger the watcher on external changes
   - **Show console:** `Never` (for clean UI)

5. Click **OK**

#### Step 2: Verify

1. Edit and save a PHP file
2. Check **Problems** tool window for rustor issues
3. File watcher runs automatically on every save

---

## Recommended Configuration

For the best experience, use **all three methods together**:

| Method | Purpose | When to Use |
|--------|---------|-------------|
| **LSP Server** | Real-time feedback as you type | Always enabled |
| **External Tool** | Detailed diff view before committing | On-demand via shortcut |
| **File Watcher** | Catch issues before commit | Auto-run on save |

---

## Integration with PHPStan

If you're migrating from PHPStan, Rustor can replace it completely:

### Replace PHPStan External Tool

**Old PHPStan config:**
```
Program: ./vendor/bin/phpstan
Arguments: analyze $FilePath$ --level 6 --memory-limit=2G
```

**New Rustor config:**
```
Program: rustor
Arguments: analyze $FilePath$ --level 6 --baseline phpstan-baseline.neon
```

**Benefits:**
- ✅ 168x faster (0.92s vs 154.5s)
- ✅ Same baseline file works
- ✅ Same error detection
- ✅ No memory limit needed

---

## Advanced Configuration

### Custom Rustor Config

Create `.rustor.toml` in your project root:

```toml
[php]
version = "8.2"

[analyze]
level = 6
baseline = "phpstan-baseline.neon"

[paths]
include = ["src/", "app/"]
exclude = ["vendor/", "tests/"]
```

Rustor will automatically use this config when run from PhpStorm.

### Multiple LSP Servers

Run Rustor alongside other PHP language servers:

**LSP4IJ Configuration:**
1. Add **Rustor** LSP server (diagnostics, refactoring)
2. Keep **Intelephense** or **PHP Language Server** (completion, navigation)

Both will work together:
- **Rustor** provides refactoring suggestions
- **Intelephense** provides code completion and go-to-definition

---

## Output Format Options

Choose the best format for your workflow:

### For External Tool

| Format | Use Case | Command |
|--------|----------|---------|
| `diff` | Code review before applying fixes | `--format diff` |
| `json` | Scripting, automation | `--format json` |
| `text` | Quick summary | `--format text` |

### For File Watcher

| Format | Use Case | Command |
|--------|----------|---------|
| `checkstyle` | PhpStorm Problems integration | `--output checkstyle` |
| `github` | GitHub Actions annotations | `--output github` |
| `sarif` | Security scanning | `--output sarif` |

---

## Troubleshooting

### Issue: LSP Server Won't Start

**Symptoms:**
- No diagnostics appear
- LSP Console shows connection errors

**Solutions:**
1. **Verify rustor installation:**
   ```bash
   which rustor
   rustor --version
   ```

2. **Test LSP manually:**
   ```bash
   echo '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{}}}' | rustor --lsp
   ```

3. **Use absolute path in PhpStorm:**
   - Don't use `rustor`, use `/usr/local/bin/rustor`
   - Or use shell wrapper: `/bin/bash -c "rustor --lsp"`

4. **Check LSP4IJ version:**
   - Update to latest version from JetBrains Marketplace

5. **Restart PhpStorm** after any configuration changes

### Issue: No Diagnostics Appear

**Symptoms:**
- LSP server starts but no issues shown

**Solutions:**
1. **Verify rustor detects issues:**
   ```bash
   rustor analyze path/to/file.php
   ```

2. **Check file is saved** - LSP works on saved files only

3. **Verify file extension** - Must be `.php`

4. **Check LSP Console** - View → Tool Windows → LSP Console

5. **Test on known issue:**
   ```php
   <?php
   $arr = [];
   array_push($arr, 1); // Should trigger rustor suggestion
   ```

### Issue: Performance Slow

**Symptoms:**
- PhpStorm freezes when opening large files
- Typing is laggy

**Solutions:**
1. **Disable File Watcher** for large files
2. **Increase PhpStorm memory:**
   - Help → Change Memory Settings → Increase to 4GB+
3. **Exclude vendor directories:**
   - Settings → Directories → Mark `vendor` as Excluded
4. **Use on-demand External Tool** instead of File Watcher

### Issue: File Watcher Not Running

**Symptoms:**
- Save file, but no output in Problems window

**Solutions:**
1. **Check watcher is enabled:**
   - Settings → Tools → File Watchers → ✅ Rustor

2. **Verify output format:**
   - Use `--output checkstyle` for PhpStorm integration

3. **Check scope:**
   - Scope should be "Project Files"
   - Not "All Files" (too broad)

4. **Test manually:**
   ```bash
   rustor analyze src/MyClass.php --output checkstyle
   ```

---

## Example Workflows

### Workflow 1: Pre-Commit Review

1. Make changes to PHP files
2. Press `Ctrl+Alt+R` (External Tool)
3. Review diff output
4. Apply fixes if needed: `rustor analyze src/ --fix`
5. Commit changes

### Workflow 2: Real-Time Development

1. Open PHP file
2. LSP server shows diagnostics in real-time
3. Hover over underlined code for details
4. Press `Alt+Enter` → Select "Fix: ..." quick fix
5. Code automatically refactored

### Workflow 3: Baseline Migration

1. Generate baseline from existing errors:
   ```bash
   rustor analyze src/ --level 6 --generate-baseline
   ```
2. Configure External Tool with `--baseline rustor-baseline.neon`
3. Only new errors appear
4. Fix new errors before committing
5. Regenerate baseline periodically

---

## Performance Tips

### For Large Projects (10K+ files)

1. **Use baseline** to ignore existing issues
2. **Analyze only changed files** using External Tool with `$FilePath$`
3. **Disable File Watcher** globally, enable only for specific directories
4. **Exclude vendor/** and test fixtures from analysis

### For Maximum Speed

**PhpStorm Configuration:**
```
Program: rustor
Arguments: analyze $FilePath$ --level 3 --no-config
Working directory: $ProjectFileDir$
```

- `--level 3` - Faster than level 6
- `--no-config` - Skip config file parsing
- Single file analysis - Faster than whole project

---

## Uninstalling

### Remove LSP Server

1. Settings → Languages & Frameworks → Language Server Protocol
2. Select "Rustor"
3. Click **-** (Remove)
4. Click **OK**

### Remove External Tool

1. Settings → Tools → External Tools
2. Select "Rustor Analyze"
3. Click **-** (Remove)
4. Click **OK**

### Remove File Watcher

1. Settings → Tools → File Watchers
2. Select "Rustor"
3. Click **-** (Remove)
4. Click **OK**

---

## See Also

- [IDE Integration](lsp.md) - Setup for other editors
- [PHPStan Migration Guide](phpstan-migration-guide.md) - Migrating from PHPStan
- [CLI Reference](cli.md) - Command-line usage
- [Configuration](configuration.md) - `.rustor.toml` options

---

**Last Updated:** 2026-01-16
**Tested With:** PhpStorm 2024.3, IntelliJ IDEA 2024.3
