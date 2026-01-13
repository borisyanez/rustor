# PHPStan Levels 7-10 Implementation Status

**Date:** 2026-01-13
**Status:** Structural implementation complete, full logic pending

## Overview

This document tracks the implementation of PHPStan analysis levels 7-10 in Rustor.

## Implementation Summary

### ✅ Level 7: Union Type Strictness
**PHPStan Config:** `checkUnionTypes: true`, `reportMaybes: true`

**Status:** Structural placeholder implemented

**Location:** `crates/rustor-analyze/src/checks/level7/`

**What it should check:**
- Methods/properties accessed on union types must exist on ALL types in the union
- Example error: `(A|B)->method()` fails if method only exists on A
- Reports "maybe" type compatibility issues instead of ignoring them

**Implementation notes:**
- Check registered in `CheckRegistry`
- Returns empty Vec (no errors reported yet)
- TODO: Implement union type parsing, tracking, and validation

---

### ✅ Level 8: Nullable Type Strictness
**PHPStan Config:** `checkNullables: true`

**Status:** Structural placeholder implemented

**Location:** `crates/rustor-analyze/src/checks/level8/`

**What it should check:**
- Accessing methods/properties on nullable types without null checks
- Example error: `function bar(?User $user) { echo $user->name; }` - $user might be null
- Requires control flow analysis to track null checks

**Implementation notes:**
- Check registered in `CheckRegistry`
- Returns empty Vec (no errors reported yet)
- TODO: Implement nullable type tracking and null-check analysis

---

### ✅ Level 9: Explicit Mixed Strictness
**PHPStan Config:** `checkExplicitMixed: true`

**Status:** Structural placeholder implemented

**Location:** `crates/rustor-analyze/src/checks/level9/`

**What it should check:**
- Explicit `mixed` types can only be passed to other `mixed` parameters
- Example error: `function baz(mixed $value) { strlen($value); }` - can't pass mixed to string
- Only applies to explicitly declared `mixed` types

**Implementation notes:**
- Check registered in `CheckRegistry`
- Returns empty Vec (no errors reported yet)
- TODO: Implement explicit mixed type detection and validation

---

### ✅ Level 10: Implicit Mixed Strictness
**PHPStan Config:** `checkImplicitMixed: true`

**Status:** Structural placeholder implemented

**Location:** `crates/rustor-analyze/src/checks/level10/`

**What it should check:**
- Missing typehints are treated as implicit `mixed`
- Same restrictions as Level 9 apply
- Example error: `function qux($value) { strlen($value); }` - implicit mixed to string

**Implementation notes:**
- Check registered in `CheckRegistry`
- Returns empty Vec (no errors reported yet)
- Level enum extended to include `Level10`
- TODO: Implement missing typehint detection and apply mixed restrictions

---

## Configuration Changes

### Level Enum (`src/config/level.rs`)

```rust
pub enum Level {
    Level0 = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
    Level5 = 5,
    Level6 = 6,
    Level7 = 7,   // NEW: Union type strictness
    Level8 = 8,   // NEW: Nullable strictness
    Level9 = 9,   // NEW: Explicit mixed
    Level10 = 10, // NEW: Implicit mixed (max)
}
```

### Level Descriptions

- Level 7: "Level 6 + union type strictness"
- Level 8: "Level 7 + nullable strictness"
- Level 9: "Level 8 + explicit mixed type checks"
- Level 10: "Strictest level: implicit mixed type checks"

### CLI Support

Users can now specify levels 7-10:
```bash
rustor analyze --level=7 src/
rustor analyze --level=8 src/
rustor analyze --level=9 src/
rustor analyze --level=10 src/
rustor analyze --level=max src/  # Now maps to Level10
```

---

## Check Registry

All 4 new checks are registered in `CheckRegistry::with_builtin_checks()`:

```rust
// Level 7 checks
registry.register(Box::new(level7::UnionTypeCheck));

// Level 8 checks
registry.register(Box::new(level8::NullableAccessCheck));

// Level 9 checks
registry.register(Box::new(level9::ExplicitMixedCheck));

// Level 10 checks
registry.register(Box::new(level10::ImplicitMixedCheck));
```

---

## Build Status

✅ All levels compile successfully
✅ No breaking changes to existing levels 0-6
✅ Full workspace builds in release mode

---

## Next Steps for Full Implementation

### Priority 1: Level 8 - Nullable Access
This is the most impactful for real-world PHP code.

**Required components:**
1. Nullable type detection from type hints (`?Type`)
2. Control flow graph for tracking null checks
3. Scope narrowing after `if ($var !== null)` checks

**Estimated effort:** 2-3 weeks

---

### Priority 2: Level 7 - Union Types
Important for modern PHP 8+ codebases.

**Required components:**
1. Union type parsing (`A|B|C`)
2. Class method/property collection (partially exists)
3. Intersection checking (member must exist on ALL types)

**Estimated effort:** 2-3 weeks

---

### Priority 3: Level 9 & 10 - Mixed Types
Less common but completes PHPStan parity.

**Required components:**
1. Explicit mixed detection in type hints
2. Implicit mixed detection (missing type hints)
3. Function signature analysis for mixed parameters
4. Type compatibility checking for mixed usage

**Estimated effort:** 1-2 weeks each

---

## Testing Strategy

### Comparison Testing Against PHPStan

For each level, create test files:

```bash
tests/phpstan_compat/
  level7_union_types.php
  level8_nullable.php
  level9_explicit_mixed.php
  level10_implicit_mixed.php
```

Run both tools and compare:
```bash
# PHPStan
./libs/vendor/bin/phpstan analyze test.php --level=7 --error-format=json > expected.json

# Rustor
rustor analyze test.php --level=7 --error-format=json > actual.json

# Compare
diff expected.json actual.json
```

---

## Technical Debt

### Current Limitations

1. **No union type support:** Level 7 check is a placeholder
2. **No nullable tracking:** Level 8 check is a placeholder
3. **No mixed type detection:** Levels 9-10 checks are placeholders
4. **No control flow analysis:** Required for accurate null checking

### Architecture Needed

1. **Type System Enhancement:**
   - Full union type representation
   - Nullable type tracking
   - Mixed vs non-mixed distinction

2. **Control Flow Graph:**
   - Required for Level 8 (null checks)
   - Also benefits Level 1 (undefined variables)
   - Enables future optimizations

3. **Symbol Table Integration:**
   - Cross-file class method/property lookup
   - Interface and trait resolution
   - Class hierarchy traversal

---

## Conclusion

The structural foundation for PHPStan levels 7-10 is complete:
- ✅ Level enums defined
- ✅ Check modules created
- ✅ Checks registered
- ✅ CLI accepts levels 7-10
- ✅ Build system works

The implementation now needs the actual analysis logic for each level. This requires significant type system and control flow enhancements, but the architecture is ready to support it.

**Current capability:** Levels 0-6 fully functional, levels 7-10 accept input but report no errors.

**Target:** Full PHPStan parity for levels 7-10 within Q1 2026.
