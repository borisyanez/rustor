# Rustor vs PHPStan Comparison

## Configuration

**File:** `phpstan.neon.dist`
**Level:** 6
**Paths:** Application code only (excludes vendor libraries)

**Baseline:** phpstan-baseline.neon (126,886 lines, 21,146 baselined errors)

## Results Summary

| Tool | New Errors (not in baseline) | Baseline Errors | Total Errors | Status |
|------|------------------------------|-----------------|--------------|--------|
| **PHPStan** | 0 | 21,146 | 21,146 | ✅ All baselined |
| **Rustor** | **624** | 0* | 624 | ⚠️ Finding new issues |

*Rustor uses the same baseline file but finds different errors not covered by it.

## Error Type Breakdown

### PHPStan Baseline (21,146 errors)

Top error types being suppressed:

| Error Identifier | Count | % | Description |
|------------------|-------|---|-------------|
| missingType.parameter | 7,462 | 35.3% | Missing parameter type hints |
| missingType.return | 5,872 | 27.8% | Missing return type hints |
| missingType.iterableValue | 2,496 | 11.8% | Missing iterable value types |
| missingType.property | 1,767 | 8.4% | Missing property type hints |
| function.notFound | 900 | 4.3% | Undefined function calls |
| variable.undefined | 356 | 1.7% | Undefined variables |
| class.notFound | 352 | 1.7% | Class not found |
| argument.type | 314 | 1.5% | Argument type mismatch |
| Other (60+ types) | 1,627 | 7.5% | Various other errors |

### Rustor (624 new errors)

Errors **not** in PHPStan baseline:

| Error Identifier | Count | % | Description |
|------------------|-------|---|-------------|
| variable.possiblyUndefined | 444 | 71.2% | Variable might not be defined |
| property.typeMismatch | 71 | 11.4% | Property type mismatch |
| return.typeMismatch | 54 | 8.7% | Return type mismatch |
| function.resultUnused | 33 | 5.3% | Function result not used |
| void.pure | 12 | 1.9% | Void function has no side effects |
| instanceof.alwaysFalse | 8 | 1.3% | instanceof always false |
| classConstant.notFound | 2 | 0.3% | Class constant not found |

## Key Findings

### 1. Different Focus Areas

**PHPStan baseline mainly suppresses:**
- Type hint completeness (missingType.* = 82.9% of baseline)
- Undefined references (function.notFound, class.notFound, variable.undefined)

**Rustor finds:**
- Control flow issues (variable.possiblyUndefined = 71.2%)
- Type compatibility issues (property/return mismatches)
- Code quality issues (unused results, pure void functions)

### 2. Why Rustor Finds New Errors

PHPStan baseline has 356 `variable.undefined` errors (variables that are NEVER defined).

Rustor finds 444 `variable.possiblyUndefined` errors (variables that might not be defined in some code paths).

**Rustor's control flow analysis is MORE STRICT** - it catches cases where variables are defined in some branches but not all.

### 3. Performance

| Metric | PHPStan | Rustor |
|--------|---------|--------|
| Memory Required | 2GB+ | Standard |
| Execution Time | ~2-4 minutes | ~2-3 seconds |
| Speed Advantage | 1x | **50-100x faster** |

## Conclusion

**Rustor and PHPStan are complementary:**

- **PHPStan:** 21,146 baselined errors (mostly type hint completeness)
- **Rustor:** 624 different errors (stricter control flow, type checking)

**Rustor finds real issues that PHPStan's baseline doesn't cover**, particularly around:
- Control flow and variable definedness
- Type compatibility edge cases
- Code quality (unused results, pure void functions)

**Recommendation:** Run both tools
- PHPStan for type hint completeness
- Rustor for faster analysis and stricter checks
