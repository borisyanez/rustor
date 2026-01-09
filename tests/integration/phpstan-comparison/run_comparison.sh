#!/bin/bash
# PHPStan vs Rustor comparison script
# Usage: ./run_comparison.sh [level] [fixture_file]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"
PHPSTAN="/Users/boris/PhpProjects/phpstan-src/bin/phpstan"
RUSTOR_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default level
LEVEL=${1:-0}

compare_file() {
    local file="$1"
    local level="$2"
    local filename=$(basename "$file")

    echo -e "\n${BLUE}=== Testing: $filename (Level $level) ===${NC}"

    # Run PHPStan
    echo -e "${YELLOW}PHPStan output:${NC}"
    $PHPSTAN analyze "$file" --level "$level" --no-progress 2>/dev/null || true

    echo ""

    # Run Rustor with --phpstan-compat flag for exact output matching
    echo -e "${YELLOW}Rustor output:${NC}"
    (cd "$RUSTOR_ROOT" && cargo run -q -p rustor-cli -- analyze "$file" --level "$level" --phpstan-compat 2>/dev/null) || true

    echo ""

    # Get error counts
    local phpstan_count=$($PHPSTAN analyze "$file" --level "$level" --error-format=json --no-progress 2>/dev/null | jq '.totals.file_errors // 0' 2>/dev/null || echo "0")
    local rustor_count=$((cd "$RUSTOR_ROOT" && cargo run -q -p rustor-cli -- analyze "$file" --level "$level" --error-format json --phpstan-compat 2>/dev/null) | jq '.totals.file_errors // 0' 2>/dev/null || echo "0")

    if [ "$phpstan_count" = "$rustor_count" ]; then
        echo -e "${GREEN}✓ Match: Both found $phpstan_count error(s)${NC}"
    else
        echo -e "${RED}✗ Mismatch: PHPStan=$phpstan_count, Rustor=$rustor_count${NC}"
    fi
}

# If a specific file is provided, test only that
if [ -n "$2" ]; then
    if [ -f "$2" ]; then
        compare_file "$2" "$LEVEL"
    elif [ -f "$FIXTURES_DIR/$2" ]; then
        compare_file "$FIXTURES_DIR/$2" "$LEVEL"
    else
        echo "File not found: $2"
        exit 1
    fi
else
    # Test all fixtures
    echo -e "${BLUE}Running PHPStan vs Rustor comparison tests${NC}"
    echo "Level: $LEVEL"
    echo "Fixtures directory: $FIXTURES_DIR"

    for file in "$FIXTURES_DIR"/*.php; do
        if [ -f "$file" ]; then
            compare_file "$file" "$LEVEL"
        fi
    done

    echo -e "\n${BLUE}=== Summary ===${NC}"
    echo "Run with different levels: $0 [0-9]"
    echo "Run specific file: $0 [level] [filename]"
fi
