# PHPStan Compatibility Progress

Use this template to track compatibility progress.

## Current Baseline

**Date**: YYYY-MM-DD
**Test Codebase**: /Users/borisyv/code/payjoy_www
**PHPStan Level**: 0

### Error Counts

| Metric | PHPStan | Rustor | Difference |
|--------|---------|--------|------------|
| Total Errors | | | |
| function.notFound | | | |
| class.notFound | | | |
| constant.notFound | | | |
| staticMethod.notFound | | | |
| Other | | | |

### Compatibility Score

```
Compatibility = (PHPStan Errors - False Negatives) / PHPStan Errors * 100
             = (X - Y) / X * 100
             = Z%
```

## Issue Categories

### False Negatives (Missing in Rustor)

| ID | Error Type | Count | Root Cause | Status |
|----|------------|-------|------------|--------|
| FN-001 | | | | |
| FN-002 | | | | |

### False Positives (Extra in Rustor)

| ID | Error Type | Count | Root Cause | Status |
|----|------------|-------|------------|--------|
| FP-001 | | | | |
| FP-002 | | | | |

## Change Log

### [Date] - Description

**Changes**:
- Implemented X
- Fixed Y

**Impact**:
- False negatives: -N
- False positives: -M
- New compatibility: Z%

---

### [Date] - Description

**Changes**:
- ...

**Impact**:
- ...

## Reproduction Commands

```bash
# Generate baseline
cd /Users/borisyv/code/payjoy_www

# PHPStan
./libs/vendor/bin/phpstan analyze \
  --configuration=phpstan.neon \
  --memory-limit=-1 \
  --error-format=json > /tmp/phpstan-baseline.json

# Rustor
/Users/borisyv/RustProjects/rustor/target/release/rustor analyze \
  -c phpstan.neon.dist \
  --format=json \
  --phpstan_compat > /tmp/rustor-baseline.json

# Compare
jq '.totals' /tmp/phpstan-baseline.json
jq '.totals' /tmp/rustor-baseline.json
```

## Next Actions

- [ ] Action item 1
- [ ] Action item 2
- [ ] Action item 3

## Notes

- Any observations, blockers, or decisions made during the process
