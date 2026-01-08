#!/usr/bin/env python3
"""
Example Rustor Plugin

This plugin demonstrates the plugin protocol:
1. Reads JSON from stdin with 'source' and 'file' fields
2. Outputs JSON with 'edits' array

Each edit has: start, end, replacement, message
"""

import json
import sys

def main():
    # Read input from stdin
    input_data = json.load(sys.stdin)
    source = input_data.get('source', '')
    file_path = input_data.get('file', '')

    edits = []

    # Example: If file doesn't start with a docblock comment, suggest adding one
    if source.startswith('<?php') and not source.startswith('<?php\n/**'):
        # Find position after <?php and any whitespace
        pos = 5  # len('<?php')
        while pos < len(source) and source[pos] in ' \t':
            pos += 1

        # Suggest adding a file docblock
        comment = "\n/**\n * @file " + file_path.split('/')[-1] + "\n */"
        edits.append({
            'start': pos,
            'end': pos,
            'replacement': comment,
            'message': 'Add file docblock comment'
        })

    # Output result
    output = {'edits': edits}
    json.dump(output, sys.stdout)

if __name__ == '__main__':
    main()
