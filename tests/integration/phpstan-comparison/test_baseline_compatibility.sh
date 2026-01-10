#!/bin/bash
# Test that rustor can read and use PHPStan-generated baselines
# This verifies baseline format compatibility between the two tools

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"
BASELINES_DIR="$SCRIPT_DIR/baselines"
RUSTOR_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
RUSTOR="$RUSTOR_ROOT/target/release/rustor"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASS=0
FAIL=0

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}PHPStan Baseline Compatibility Test${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Test each fixture with its PHPStan-generated baseline
for fixture in "$FIXTURES_DIR"/*.php; do
    filename=$(basename "$fixture" .php)
    baseline="$BASELINES_DIR/${filename}_baseline.neon"

    # Extract level from filename
    level=$(echo "$filename" | grep -o 'level[0-9]' | grep -o '[0-9]')
    if [ -z "$level" ]; then level=0; fi

    echo -e "${YELLOW}Testing: $filename (level $level)${NC}"

    if [ ! -f "$baseline" ]; then
        echo -e "  ${RED}✗ Baseline file not found: $baseline${NC}"
        ((FAIL++))
        continue
    fi

    # First, run without baseline to get error count
    errors_without=$("$RUSTOR" analyze "$fixture" --level "$level" --error-format json 2>/dev/null | jq '.totals.file_errors // 0' 2>/dev/null || echo "0")

    # Then run with PHPStan baseline
    errors_with=$("$RUSTOR" analyze "$fixture" --level "$level" --baseline "$baseline" --error-format json 2>/dev/null | jq '.totals.file_errors // 0' 2>/dev/null || echo "?")

    # Check if baseline was parsed (errors_with should be 0 or less than errors_without)
    if [ "$errors_with" = "?" ]; then
        echo -e "  ${RED}✗ Failed to parse baseline${NC}"
        ((FAIL++))
    elif [ "$errors_with" -eq 0 ] || [ "$errors_with" -lt "$errors_without" ]; then
        echo -e "  ${GREEN}✓ Baseline works: $errors_without errors → $errors_with errors${NC}"
        ((PASS++))
    else
        echo -e "  ${RED}✗ Baseline not applied: still $errors_with errors (expected < $errors_without)${NC}"
        ((FAIL++))
    fi
done

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Passed: $PASS${NC}"
echo -e "${RED}Failed: $FAIL${NC}"

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}All baseline compatibility tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed!${NC}"
    exit 1
fi
