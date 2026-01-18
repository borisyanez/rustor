# Phase 5: 100% PHPStan Compatibility Plan

## Implementation Results (2026-01-18)

### Final Status: 74% Reduction Achieved

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Rustor errors | 74 | 19 | **-55 (74%)** |
| PHPStan errors | 53 | 53 | unchanged |

### Implemented Features

| Feature | Status | Impact |
|---------|--------|--------|
| **Task 1: isset() control flow** | ✅ Complete | |
| - Positive `isset($var)` in if body | ✅ | ~20 errors |
| - `isset($var['key'])` extracts base var | ✅ | |
| - Short-circuit `&&`: `isset($var) && $var['key']` | ✅ | ~6 errors |
| - Ternary: `isset($var) ? $var : default` | ✅ | ~3 errors |
| **Task 2: !empty() control flow** | ✅ Complete | ~3 errors |
| **Task 3: Condition correlation** | ✅ Complete | ~3 errors |
| - Same condition guards assignment and usage | ✅ | |
| - Condition text hashing for correlation | ✅ | |
| **Task 4: Null coalesce** | ✅ Already done | (prior work) |
| **Additional fixes** | ✅ Complete | |
| - `list($a, $b) = $array` destructuring | ✅ | ~11 errors |
| - Short array `[$a, $b] = $array` (PHP 7.1+) | ✅ | ~7 errors |
| - Switch case early exit (any stmt) | ✅ | ~2 errors |
| - Reference foreach `&$var` | ✅ | ~1 error |
| - Negated isset else: `if(!isset($var)) else use($var)` | ✅ | ~1 error |

### Remaining 19 Errors

Most are the **inverse condition** pattern which requires more sophisticated analysis:
```php
if ($condition) { $var = ...; }
if (!$condition || $var === ...) { $var = ...; }
use($var);  // Always defined but hard to prove statically
```

### Commit
- `d8318e5`: feat(undefined-variable): Enhance control flow analysis for 74% error reduction

---

## Original Plan (for reference)

### Initial State (as of 2026-01-18)

- **Rustor errors**: 74 (with `--ignore-baseline-counts` and PHPStan baseline)
- **PHPStan errors**: 53 (all "unknown class" errors, different type)
- **Gap**: 74 false positives where Rustor reports `variable.undefined` but PHPStan doesn't

## Root Cause Analysis

All 74 false positives are `variable.undefined` errors where PHPStan's control flow analysis understands the variable IS defined, but Rustor doesn't.

### Pattern Categories

#### Pattern 1: `isset()` implies variable definition (~50% of cases)
```php
if ($apiKey !== '') {
    $adminRow = getAdminRowFromApiRequest();
}
// ...
if (isset($adminRow['financeProviderId'])) {  // isset() means $adminRow IS defined
    echo $adminRow['financeProviderId'];      // Rustor FALSE POSITIVE
}
```

**Also covers:**
- `isset($var)` - variable must be defined
- `isset($var['key'])` - variable must be defined (and be array-like)
- `isset($var->prop)` - variable must be defined (and be object-like)

#### Pattern 2: `!empty()` implies definition (~10% of cases)
```php
if (!empty($var)) {    // Implies $var is defined AND truthy
    use($var);         // Rustor FALSE POSITIVE
}
```

#### Pattern 3: Condition correlation (~30% of cases)
```php
if ($condition) {
    $var = getValue();
}
// ... other code ...
if ($condition) {      // Same condition = $var is defined in this branch
    use($var);         // Rustor FALSE POSITIVE
}
```

#### Pattern 4: Null coalesce as guard (~10% of cases)
```php
$var = $maybeUndefined ?? 'default';  // $var is always defined after this
```

---

## Implementation Plan

### Task 1: isset() Control Flow Enhancement
**File**: `crates/rustor-analyze/src/checks/level1/undefined_variable.rs`
**Complexity**: Medium
**Impact**: ~50% reduction in false positives

**Implementation:**
1. When visiting an `if` statement with `isset($var)` or `isset($var['key'])` condition:
   - Mark `$var` as "conditionally defined" within the true branch
   - Track that the variable exists (even if value unknown)

2. Modify `is_variable_defined()` to check:
   - Direct assignments (current behavior)
   - isset() condition scopes (new)

**Key code location:**
```rust
// In UndefinedVariableVisitor
fn visit_if_statement(&mut self, if_stmt: &IfStatement<'a>, source: &str) -> bool {
    // Check if condition is isset($var) or isset($var['key'])
    if let Some(var_name) = self.extract_isset_variable(&if_stmt.condition) {
        // Mark variable as defined within the if-body scope
        self.scopes.last_mut().unwrap().defined_vars.insert(var_name);
    }
    true
}
```

### Task 2: !empty() Control Flow Enhancement
**File**: `crates/rustor-analyze/src/checks/level1/undefined_variable.rs`
**Complexity**: Low
**Impact**: ~10% reduction

**Implementation:**
Same as Task 1, but for `!empty($var)` conditions.

```rust
fn extract_empty_check_variable(&self, expr: &Expression<'_>) -> Option<String> {
    // Match: !empty($var)
    if let Expression::UnaryPrefix(op) = expr {
        if op.operator == "!" {
            if let Expression::FunctionCall(call) = &op.operand {
                if call.name == "empty" && call.arguments.len() == 1 {
                    // Extract variable name from first argument
                }
            }
        }
    }
    None
}
```

### Task 3: Condition Correlation Tracking
**File**: `crates/rustor-analyze/src/checks/level1/undefined_variable.rs`
**Complexity**: High
**Impact**: ~30% reduction

**Implementation:**
1. Create a `ConditionTracker` that remembers:
   - Which variables are assigned under which conditions
   - Map of `condition_hash -> Vec<assigned_vars>`

2. When checking variable usage:
   - If current scope is inside `if (condition)`
   - And `condition` previously guarded an assignment to `$var`
   - Then `$var` is considered defined

**Data structure:**
```rust
struct ConditionTracker {
    // Hash of condition expression -> variables assigned under it
    condition_assignments: HashMap<u64, HashSet<String>>,
}

impl ConditionTracker {
    fn record_assignment(&mut self, condition: &Expression, var_name: &str) {
        let hash = self.hash_condition(condition);
        self.condition_assignments
            .entry(hash)
            .or_default()
            .insert(var_name.to_string());
    }

    fn is_defined_under_condition(&self, condition: &Expression, var_name: &str) -> bool {
        let hash = self.hash_condition(condition);
        self.condition_assignments
            .get(&hash)
            .map(|vars| vars.contains(var_name))
            .unwrap_or(false)
    }
}
```

### Task 4: Null Coalesce Handling
**File**: `crates/rustor-analyze/src/checks/level1/undefined_variable.rs`
**Complexity**: Low
**Impact**: ~10% reduction

**Implementation:**
When seeing `$var ?? default`, don't report `$var` as undefined - the `??` operator handles it.

```rust
fn visit_binary_expression(&mut self, expr: &BinaryExpression<'a>, source: &str) -> bool {
    if expr.operator == "??" {
        // Left side of ?? is allowed to be undefined
        // Only check right side for undefined variables
        self.visit_expression(&expr.right, source);
        return false; // Don't recurse into left side
    }
    true
}
```

---

## Testing Strategy

### Test Command
```bash
cd /Users/borisyv/code/payjoy_www && \
/Users/borisyv/RustProjects/rustor/target/release/rustor analyze \
  -c phpstan.neon.dist \
  --phpstan-compat \
  --baseline phpstan-baseline.neon \
  --ignore-baseline-counts
```

### Success Criteria
- 0 errors with PHPStan baseline
- All errors should match PHPStan's output exactly

### Intermediate Milestones
| After Task | Expected Errors |
|------------|-----------------|
| Current    | 74              |
| Task 1     | ~37             |
| Task 2     | ~30             |
| Task 3     | ~7              |
| Task 4     | 0               |

---

## Files Modified

1. **Primary**: `crates/rustor-analyze/src/checks/level1/undefined_variable.rs`
   - Added isset/empty condition tracking
   - Added condition correlation tracking
   - Added list/array destructuring support
   - Added negated isset else branch handling
   - ~500 lines added

---

## Sample False Positives for Testing

```
api/partner/util.php:970 - $adminRow (isset pattern)
api/ping.php:41 - $notificationParams (isset pattern)
creditline-consumer/util/utils.php:286 - $incodeInfo (isset pattern)
tools/secure/etl/run-git-etl.php:31 - $startDate (condition correlation)
```

---

## Commit History

- Commit `d8318e5`: Phase 5 implementation - 74% error reduction (this work)
- Commit `f2376e0`: Added isset and null coalesce control flow tracking (prior)
- Commit `6c2955d`: Added --ignore-baseline-counts flag

## Future Work (Remaining 19 Errors)

To reach 0 errors, would need to implement:
1. **Inverse condition detection**: Recognize `$x` vs `!$x` as complementary
2. **OR condition analysis**: `!$cond || $var === ...` implies `$var` defined if first part false
3. **Cross-branch data flow**: Track variable state across non-adjacent if blocks
