# Rustor Plugin System

Rustor supports external plugins for custom rules. Plugins are executables that communicate via JSON over stdin/stdout.

## Plugin Location

Plugins are discovered from `~/.rustor/plugins/`. Each plugin is a directory containing a `plugin.toml` manifest.

## Plugin Manifest

Create `plugin.toml` in your plugin directory:

```toml
name = "my-plugin"
version = "1.0.0"
description = "My custom PHP rule"
command = "python3"
args = ["plugin.py"]
min_php_version = "8.0"  # Optional
category = "custom"      # Optional
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique plugin identifier |
| `version` | Yes | Plugin version |
| `description` | No | Human-readable description |
| `command` | Yes | Executable to run |
| `args` | No | Arguments to pass |
| `min_php_version` | No | Minimum PHP version |
| `category` | No | Rule category |

## Plugin Protocol

### Input (stdin)

Plugins receive JSON on stdin:

```json
{
  "source": "<?php\necho 'hello';",
  "file": "/path/to/file.php"
}
```

### Output (stdout)

Plugins output JSON with suggested edits:

```json
{
  "edits": [
    {
      "start": 6,
      "end": 10,
      "replacement": "print",
      "message": "Use print instead of echo"
    }
  ]
}
```

To report an error:

```json
{
  "edits": [],
  "error": "Failed to analyze file"
}
```

### Edit Fields

| Field | Required | Description |
|-------|----------|-------------|
| `start` | Yes | Byte offset where edit begins |
| `end` | Yes | Byte offset where edit ends |
| `replacement` | Yes | Text to insert |
| `message` | No | Description of the change |

## Example Plugins

### Python Plugin

```python
#!/usr/bin/env python3
import json
import sys

def main():
    input_data = json.load(sys.stdin)
    source = input_data['source']
    file_path = input_data['file']

    edits = []

    # Your analysis logic here
    # Example: Find 'echo' and suggest 'print'
    pos = source.find('echo')
    if pos != -1:
        edits.append({
            'start': pos,
            'end': pos + 4,
            'replacement': 'print',
            'message': 'Use print instead of echo'
        })

    json.dump({'edits': edits}, sys.stdout)

if __name__ == '__main__':
    main()
```

### Node.js Plugin

```javascript
#!/usr/bin/env node
const fs = require('fs');

let input = '';
process.stdin.on('data', chunk => input += chunk);
process.stdin.on('end', () => {
    const data = JSON.parse(input);
    const { source, file } = data;

    const edits = [];

    // Your analysis logic here

    console.log(JSON.stringify({ edits }));
});
```

### Bash Plugin (using jq)

```bash
#!/bin/bash
# Simple plugin using jq

read input
source=$(echo "$input" | jq -r '.source')
file=$(echo "$input" | jq -r '.file')

# Your analysis logic here
echo '{"edits": []}'
```

## Installing Plugins

1. Create the plugins directory:
   ```bash
   mkdir -p ~/.rustor/plugins
   ```

2. Copy your plugin:
   ```bash
   cp -r my-plugin ~/.rustor/plugins/
   ```

3. Verify it works:
   ```bash
   echo '{"source":"<?php echo 1;","file":"test.php"}' | ~/.rustor/plugins/my-plugin/plugin.py
   ```

## Tips

1. **Parse carefully**: PHP source can contain any UTF-8 characters
2. **Use byte offsets**: Start/end positions are byte offsets, not character positions
3. **Overlapping edits**: Avoid suggesting overlapping edits
4. **Performance**: Keep plugins fast - they're called per file
5. **Testing**: Test with various PHP files before deploying

## Future Plans

- CLI flag `--plugin <name>` to enable specific plugins
- Plugin marketplace/registry
- WebAssembly plugin support for cross-platform sandboxed execution
