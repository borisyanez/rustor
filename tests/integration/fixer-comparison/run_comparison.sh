#!/bin/bash
# Integration test: Compare rustor fixer output with PHP-CS-Fixer
#
# Prerequisites:
# - php-cs-fixer installed: composer global require friendsofphp/php-cs-fixer
# - rustor built: cargo build --release
#
# Usage:
#   ./run_comparison.sh [--verbose]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUSTOR_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
RUSTOR_BIN="$RUSTOR_ROOT/target/release/rustor"

# Check prerequisites
if ! command -v php-cs-fixer &> /dev/null; then
    echo "Error: php-cs-fixer not found. Install with: composer global require friendsofphp/php-cs-fixer"
    exit 1
fi

if [ ! -f "$RUSTOR_BIN" ]; then
    echo "Error: rustor binary not found. Build with: cargo build --release"
    exit 1
fi

# Create temp directories
TEMP_DIR=$(mktemp -d)
mkdir -p "$TEMP_DIR/php-cs-fixer" "$TEMP_DIR/rustor"

# Copy original files
cp "$SCRIPT_DIR/original/"*.php "$TEMP_DIR/php-cs-fixer/"
cp "$SCRIPT_DIR/original/"*.php "$TEMP_DIR/rustor/"

# Create config for temp directory
cat > "$TEMP_DIR/.php-cs-fixer.php" << 'EOF'
<?php

use PhpCsFixer\Config;
use PhpCsFixer\Finder;

$finder = Finder::create()
    ->in(__DIR__)
    ->name('*.php');

return (new Config())
    ->setRiskyAllowed(true)
    ->setRules([
        '@PSR12' => true,
        'braces_position' => [
            'functions_opening_brace' => 'same_line',
            'classes_opening_brace' => 'same_line',
            'control_structures_opening_brace' => 'same_line',
        ],
        'single_quote' => true,
    ])
    ->setFinder($finder);
EOF

# Run PHP-CS-Fixer
echo "Running PHP-CS-Fixer..."
php-cs-fixer fix "$TEMP_DIR/php-cs-fixer/" --config="$TEMP_DIR/.php-cs-fixer.php" --quiet 2>/dev/null || true

# Run rustor
echo "Running rustor..."
"$RUSTOR_BIN" --fixer --fixer-config "$TEMP_DIR/.php-cs-fixer.php" "$TEMP_DIR/rustor/"*.php --fix 2>/dev/null || true

# Compare results
echo ""
echo "=== Comparison Results ==="
echo ""

identical=0
different=0
different_files=""

for f in "$SCRIPT_DIR/original/"*.php; do
    name=$(basename "$f")
    if diff -q "$TEMP_DIR/php-cs-fixer/$name" "$TEMP_DIR/rustor/$name" > /dev/null 2>&1; then
        echo "✅ $name - IDENTICAL"
        ((identical++))
    else
        echo "❌ $name - DIFFERENT"
        ((different++))
        different_files="$different_files $name"
    fi
done

echo ""
echo "Summary: $identical identical, $different different"

# Show diffs if --verbose
if [ "$1" = "--verbose" ] && [ $different -gt 0 ]; then
    echo ""
    echo "=== Differences ==="
    for name in $different_files; do
        echo ""
        echo "--- $name ---"
        diff "$TEMP_DIR/php-cs-fixer/$name" "$TEMP_DIR/rustor/$name" || true
    done
fi

# Cleanup
rm -rf "$TEMP_DIR"

# Exit with error if there are differences (useful for CI)
if [ $different -gt 0 ]; then
    exit 1
fi
