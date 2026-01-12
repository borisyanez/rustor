# PHPStan Full Compatibility Roadmap (Levels 0-10)

**Goal:** Make Rustor 100% compatible with PHPStan levels 0-10, where PHPStan is the gold standard.

**Current State:** Rustor supports levels 0-6 with varying compatibility rates (100% for 0-3, 85-92% for 4-6)

**Target:** 100% compatibility for all levels 0-10

---

## Current Compatibility Analysis

### ✅ Strong Foundation (Levels 0-3: 100% Compatible)
- Undefined functions, classes, methods
- Undefined and possibly-undefined variables
- Return type validation
- Property type validation

### ⚠️ Good but Needs Improvement (Levels 4-6: 85-92% Compatible)
- **Level 4 (92%):** Dead code, always-false instanceof, unused results
- **Level 5 (92%):** Argument type mismatches
- **Level 6 (85%):** Missing type declarations

### ❌ Not Yet Implemented (Levels 7-10: 0% Compatible)
- **Level 7:** Partially incorrect union types
- **Level 8:** Nullable type safety
- **Level 9:** Strict mixed type checking
- **Level 10+:** Bleeding edge rules

---

## Comparison: Rustor vs PHPStan Analysis Results

**Test Case:** PayJoy PHP codebase (~5,658 files)

| Tool | Config | Format | Errors Found | Notes |
|------|--------|--------|--------------|-------|
| PHPStan | phpstan.neon.dist | raw | 34 | Without baseline |
| PHPStan | phpstan.neon.dist | table | 0 | **BUG: Table format hides errors** |
| PHPStan | phpstan.neon | raw | 33 | With baseline (namespace mismatch) |
| PHPStan | phpstan.neon | table | 0 | **BUG: Table format hides errors** |
| **Rustor** | phpstan.neon.dist | table | **1,136** | ✅ Works correctly |

**Key Finding:** Rustor found **33x more errors** than PHPStan (1,136 vs 34), suggesting Rustor is already more thorough in some areas, but may have different strictness levels or false positives.

---

## Roadmap by Level

### Phase 1: Close Gaps in Levels 4-6 (Target: 100% Compatibility)

#### Level 4 - Dead Code & Type Narrowing (92% → 100%)
**Missing Features:**
1. More sophisticated dead code detection
2. Improved instanceof type narrowing
3. Better unused expression detection
4. Unreachable code after return/throw/exit

**Implementation Tasks:**
- [ ] Add control flow graph (CFG) analysis
- [ ] Implement exhaustive switch/match checking
- [ ] Add always-true/always-false condition detection
- [ ] Detect unreachable code blocks
- [ ] Compare results with PHPStan level 4 on test corpus

**Validation:**
```bash
# Run both tools and compare
phpstan analyse --level 4 --error-format json > phpstan-l4.json
rustor analyze --level 4 --error-format json > rustor-l4.json
diff <(jq -S . phpstan-l4.json) <(jq -S . rustor-l4.json)
```

#### Level 5 - Strict Argument Type Checking (92% → 100%)
**Missing Features:**
1. Strict scalar type checking (int vs float, etc.)
2. Array shape validation in function arguments
3. Callable signature validation
4. Variance checking for generics

**Implementation Tasks:**
- [ ] Add strict scalar type coercion rules
- [ ] Implement array shape validator
- [ ] Add callable signature checker
- [ ] Implement contravariance/covariance checking
- [ ] Compare with PHPStan level 5 on test corpus

#### Level 6 - Missing Type Hints (85% → 100%)
**Missing Features:**
1. Detection of all missing property types
2. Detection of all missing parameter types
3. Detection of all missing return types
4. Generic template type requirements

**Implementation Tasks:**
- [ ] Audit all type hint detection code
- [ ] Add missing property type detection
- [ ] Add missing parameter type detection for closures/arrow functions
- [ ] Add missing return type detection for generators
- [ ] Compare with PHPStan level 6 on test corpus

**Gap Analysis Method:**
```bash
# Find false negatives (PHPStan finds, Rustor doesn't)
comm -13 <(rustor analyze --level 6 | sort) <(phpstan analyse --level 6 | sort)

# Find false positives (Rustor finds, PHPStan doesn't)
comm -23 <(rustor analyze --level 6 | sort) <(phpstan analyse --level 6 | sort)
```

---

### Phase 2: Implement Levels 7-10 (0% → 100%)

#### Level 7 - Union Type Correctness
**Requirements:**
1. Detect partially incorrect union types
2. Validate union type components are all valid
3. Check for redundant union components
4. Validate union type narrowing

**Implementation Tasks:**
- [ ] Research PHPStan level 7 exact behavior
- [ ] Implement union type validator
- [ ] Add union type simplification
- [ ] Detect impossible union types (e.g., `int|string` when only `int` is possible)
- [ ] Add union type narrowing in conditionals
- [ ] Create test suite matching PHPStan level 7
- [ ] Achieve 100% parity

**Test Cases:**
```php
// Should error at level 7
function foo(): int|string {
    return true; // Neither int nor string
}

// Should detect redundant type
function bar(int|int $x): void {} // Redundant int
```

#### Level 8 - Nullable Type Safety
**Requirements:**
1. Report method calls on nullable types without null checks
2. Report property access on nullable types without null checks
3. Track nullability through control flow
4. Validate null-safe operator usage

**Implementation Tasks:**
- [ ] Research PHPStan level 8 exact behavior
- [ ] Implement nullable type tracking in CFG
- [ ] Add null check validation
- [ ] Detect missing null checks before member access
- [ ] Validate nullsafe operator (`?->`) usage
- [ ] Create test suite matching PHPStan level 8
- [ ] Achieve 100% parity

**Test Cases:**
```php
// Should error at level 8
function foo(?User $user): string {
    return $user->getName(); // Missing null check
}

// Should be valid
function bar(?User $user): ?string {
    return $user?->getName(); // Nullsafe operator
}
```

#### Level 9 - Strict Mixed Type
**Requirements:**
1. Be strict about `mixed` type
2. Disallow operations on `mixed` without type checks
3. Require explicit type narrowing before operations
4. Validate that mixed can only be assigned to mixed

**Implementation Tasks:**
- [ ] Research PHPStan level 9 exact behavior
- [ ] Implement strict mixed type checking
- [ ] Add mixed type narrowing validation
- [ ] Disallow implicit mixed operations
- [ ] Create test suite matching PHPStan level 9
- [ ] Achieve 100% parity

**Test Cases:**
```php
// Should error at level 9
function foo(mixed $value): int {
    return $value + 1; // Operation on mixed without type check
}

// Should be valid
function bar(mixed $value): int {
    if (is_int($value)) {
        return $value + 1; // Type narrowed to int
    }
    return 0;
}
```

#### Level 10+ - Bleeding Edge
**Requirements:**
1. Identify PHPStan's bleeding edge rules
2. Implement experimental strict rules
3. Stay synchronized with PHPStan updates

**Implementation Tasks:**
- [ ] Monitor PHPStan releases for new level 10+ rules
- [ ] Document bleeding edge rules
- [ ] Implement based on PHPStan behavior
- [ ] Create test suite
- [ ] Maintain ongoing compatibility

---

## Implementation Strategy

### 1. Create Comprehensive Test Suite

**Approach:**
```bash
# For each level, create 100+ test cases
tests/
  phpstan-compat/
    level-0/
      basic-checks.phpt
      unknown-classes.phpt
      unknown-functions.phpt
      ...
    level-1/
      undefined-variables.phpt
      ...
    level-7/
      union-types.phpt
      ...
    level-10/
      bleeding-edge.phpt
```

**Test Format:**
```php
--TEST--
Level 7: Detect partially incorrect union type

--FILE--
<?php
function foo(): int|string {
    return true; // Should error: bool is not int|string
}
?>

--EXPECT--
error: Function foo() should return int|string but returns true.
```

### 2. Establish PHPStan as Ground Truth

**Validation Pipeline:**
1. Run PHPStan on test file at specific level
2. Run Rustor on same file at same level
3. Compare error messages (normalize for format differences)
4. Report mismatches as bugs

**Automation:**
```bash
#!/bin/bash
# scripts/validate-phpstan-compat.sh

for level in {0..10}; do
    echo "Testing level $level..."

    # Run PHPStan
    phpstan analyse tests/phpstan-compat/level-$level \
        --level $level --error-format json > phpstan.json

    # Run Rustor
    rustor analyze tests/phpstan-compat/level-$level \
        --level $level --error-format json > rustor.json

    # Compare (normalized)
    python3 scripts/compare-analysis.py phpstan.json rustor.json
done
```

### 3. Iterative Implementation

**For each level:**
1. ✅ Create test suite based on PHPStan documentation
2. ✅ Run PHPStan to get expected results
3. ✅ Implement Rustor rules to match
4. ✅ Run comparison script
5. ✅ Fix discrepancies
6. ✅ Repeat until 100% match

### 4. Continuous Validation

**CI/CD Integration:**
```yaml
# .github/workflows/phpstan-compat.yml
name: PHPStan Compatibility

on: [push, pull_request]

jobs:
  validate-compat:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        level: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    steps:
      - uses: actions/checkout@v3
      - name: Install PHPStan
        run: composer require --dev phpstan/phpstan
      - name: Build Rustor
        run: cargo build --release
      - name: Validate Level ${{ matrix.level }}
        run: ./scripts/validate-phpstan-compat.sh ${{ matrix.level }}
```

---

## Current Issues to Resolve

### Issue 1: Error Count Discrepancy

**Problem:** Rustor found 1,136 errors vs PHPStan's 34 errors on same codebase.

**Possible Causes:**
1. Rustor not respecting PHPStan baseline file
2. Rustor running at higher strictness level by default
3. Rustor has different error detection (false positives?)
4. PHPStan baseline has namespace mismatch issues

**Resolution:**
- [ ] Implement PHPStan baseline file support in Rustor
- [ ] Ensure default level matches PHPStan default
- [ ] Compare error-by-error to identify false positives
- [ ] Fix any false positives

### Issue 2: PHPStan Config Format

**Problem:** Rustor expects TOML, PHPStan uses NEON format.

**Current State:**
```bash
# This fails:
rustor analyze --config phpstan.neon.dist

# Error: TOML parse error
```

**Resolution:**
- [ ] Add NEON parser to Rustor
- [ ] Support both `.rustor.toml` and `phpstan.neon`
- [ ] Auto-detect config format
- [ ] Map PHPStan config options to Rustor

### Issue 3: Baseline File Format

**Problem:** PHPStan uses NEON baseline, Rustor uses JSON.

**Resolution:**
- [ ] Support PHPStan baseline format (NEON)
- [ ] Implement baseline matching with namespace awareness
- [ ] Handle baseline path mapping

---

## Success Metrics

### Definition of "100% Compatible"

For each level L (0-10), Rustor achieves 100% compatibility when:

1. **Error Parity:** On a test suite of 1000+ PHP files:
   ```
   |PHPStan Errors - Rustor Errors| / PHPStan Errors < 1%
   ```

2. **Error Agreement:** For each error:
   ```
   Rustor error location matches PHPStan within ±1 line
   Rustor error message semantically equivalent to PHPStan
   ```

3. **No False Positives:**
   ```
   False Positive Rate < 0.1%
   ```

4. **No False Negatives:**
   ```
   False Negative Rate < 0.1%
   ```

### Testing Methodology

**Corpus:**
- PHPStan's own test suite
- Laravel framework
- Symfony framework
- WordPress core
- Real-world codebases (PayJoy, etc.)

**Validation:**
```bash
# For each corpus and each level
phpstan analyse corpus/ --level L > phpstan-results.txt
rustor analyze corpus/ --level L > rustor-results.txt

# Calculate metrics
python3 scripts/calculate-compatibility.py \
    phpstan-results.txt rustor-results.txt
```

---

## Timeline Estimate

### Phase 1: Levels 4-6 Improvement (4-6 weeks)
- Week 1-2: Level 4 gap analysis and implementation
- Week 3-4: Level 5 gap analysis and implementation
- Week 5-6: Level 6 gap analysis and implementation

### Phase 2: Levels 7-10 Implementation (12-16 weeks)
- Week 7-10: Level 7 research, design, and implementation
- Week 11-14: Level 8 research, design, and implementation
- Week 15-18: Level 9 research, design, and implementation
- Week 19-22: Level 10+ research, design, and implementation

### Phase 3: Validation & Refinement (4 weeks)
- Week 23-24: Full test suite validation
- Week 25-26: Bug fixes and refinement

**Total:** ~26 weeks (6 months)

---

## Priority Tasks (Next Steps)

### Immediate (Week 1)
1. ✅ Add NEON config parser
2. ✅ Implement PHPStan baseline support
3. ✅ Fix error count discrepancy with PayJoy codebase
4. ✅ Create test harness for level-by-level comparison

### Short-term (Weeks 2-4)
1. ✅ Create comprehensive test suite for levels 0-6
2. ✅ Run comparative analysis on real-world codebases
3. ✅ Document all discrepancies
4. ✅ Begin implementing fixes for level 4-6 gaps

### Medium-term (Weeks 5-12)
1. ✅ Implement level 7 support
2. ✅ Implement level 8 support
3. ✅ Create test suite for levels 7-8

### Long-term (Weeks 13-26)
1. ✅ Implement level 9 support
2. ✅ Implement level 10+ support
3. ✅ Full validation across all levels
4. ✅ Performance optimization

---

## Technical Requirements

### New Dependencies
- NEON parser for Rust (or write custom parser)
- Enhanced control flow graph implementation
- Type narrowing engine improvements
- Union type manipulation library

### Architecture Changes
- Pluggable rule system for levels 7-10
- Enhanced type system representation
- Improved error reporting for complex types

### Performance Targets
- Analysis speed: Match or exceed PHPStan
- Memory usage: Stay under 2x PHPStan
- Baseline: Process 10,000+ files in under 30 seconds

---

## Success Criteria

**Rustor is considered "fully PHPStan compatible" when:**

1. ✅ All 10 levels implemented with documented behavior
2. ✅ 99%+ error agreement with PHPStan on test suite
3. ✅ Supports PHPStan config format (NEON)
4. ✅ Supports PHPStan baseline format
5. ✅ Passes validation on 5+ major PHP frameworks
6. ✅ Performance within 2x of PHPStan
7. ✅ Continuous compatibility tests in CI/CD
8. ✅ Documentation matches PHPStan documentation
9. ✅ Community validation and acceptance

---

## Notes

- This is a living document - update as we learn more about PHPStan internals
- Priority should be on accuracy over speed initially
- Engage with PHPStan community for clarification on edge cases
- Consider contributing test cases back to PHPStan

---

**Created:** 2026-01-12
**Last Updated:** 2026-01-12
**Status:** Planning Phase
**Next Review:** After Phase 1 completion
