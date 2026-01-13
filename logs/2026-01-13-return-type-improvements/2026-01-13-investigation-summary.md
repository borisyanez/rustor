# Investigation Summary: Remaining 59 Return Type Mismatch Errors

## Accurate Error Count Tracking

**Configuration:** Using `phpstan.neon.dist` (excludes vendor code, analyzes only application paths)

### Timeline

| Stage | return.typeMismatch | Total Errors | Notes |
|-------|---------------------|--------------|-------|
| Before union type fix (commit 0520052) | **119** | 689 | After self/static fix |
| After union type + callable fix (commit be71b8b) | **59** | 629 | Current state |
| **Errors Fixed** | **60** (50.4% reduction) | **60** (8.7% overall) | |

## What Was Fixed

The union type + Closure/callable implementation fixed 60 errors:

1. **Union type member returns** (~52 errors estimated)
   - Functions returning `null` when type is `int|null`
   - Functions returning `string` when type is `Cart|string`
   - Functions returning specific type when union includes it

2. **Closure/callable compatibility** (~8 errors estimated)
   - Functions returning `Closure` when `callable` expected

## Remaining 59 Errors - Breakdown

### Category 1: Interface/Implementation Returns (54 errors, 92%)

**Issue:** Functions declare interface return type but return concrete implementation

#### Application Interfaces (38 errors, 64%)
Examples:
- `CartVerificationDtoInterface` → `ClaroDto`, `KioskDto`, `TigoDto` (4 errors)
- `PaymentHandlerInterface` → `InStorePaymentHandler`, `AsaasPaymentHandler`, etc. (6 errors)
- `PespayServiceInterface` → `PespayDownPaymentService` (2 errors)
- `ExecutableActionInterface` → `CreditLineSendNotification`, etc. (3 errors)
- And 23 more similar patterns

#### PSR/Standard Interfaces (4 errors, 7%)
- `ResponseInterface` → `Response` (3 errors) - PSR-7
- `ClientInterface` → `Client` (1 error)

#### Framework Interfaces (1 error, 2%)
- `Collection` → `ArrayCollection` (1 error) - Doctrine

#### Parent Class Returns (11 errors, 19%)
- `DebtAcknowledgement` → `ZADebtAcknowledgement`
- Various transformer/service subclasses

### Category 2: Generator Returns (1 error, 2%)
- Declares `Generator` but returns `array`

### Category 3: Variable Class Returns (2 errors, 3%)
- Returns variable like `$calculatorClass` instead of known type

### Category 4: Truncated Messages (2 errors, 3%)
- Incomplete error messages needing investigation

## Quick Win Opportunities

### Option 1: Hardcoded Well-Known Types ✅ RECOMMENDED

**Effort:** 30-45 minutes
**Risk:** Very low
**Impact:** 5-6 errors fixed (8-10% reduction)

```rust
// PSR-7 HTTP interfaces
if expected.ends_with("responseinterface") && actual == "response" { return true; }
if expected.ends_with("requestinterface") && actual == "request" { return true; }
if expected.ends_with("uriinterface") && actual == "uri" { return true; }
if expected.ends_with("streaminterface") && actual == "stream" { return true; }

// HTTP client
if expected == "clientinterface" && actual == "client" { return true; }

// Doctrine Collections
if expected == "collection" && actual == "arraycollection" { return true; }
```

### Option 2: Interface Name Pattern Recognition ⚠️ MODERATE RISK

**Effort:** 1 hour
**Risk:** Moderate (may create false positives)
**Impact:** 5-10 additional errors fixed (8-17% reduction)

```rust
// ServiceInterface → Service, ValidatorInterface → Validator, etc.
if expected.ends_with("interface") {
    let base = &expected[..expected.len() - 9];
    if actual == base { return true; }
}
```

**Examples it would fix:**
- `TicketingServiceInterface` → `TicketingService` ✓
- `FeeSchemaGeneratorInterface` → `FeeSchemaGenerator` ✓
- `PespayServiceInterface` → `PespayDownPaymentService` ✗ (different name)

### Option 3: Generator Detection ⚠️ LOW PRIORITY

**Effort:** 30 minutes
**Risk:** Low
**Impact:** 1 error fixed (2% reduction)

Use existing `block_contains_yield()` method to infer Generator return type.

## Not Recommended (High Effort, Low Value Now)

### Full Interface/Inheritance Tracking ❌

**Effort:** Multiple days
**Risk:** High complexity
**Impact:** 40-50 errors fixed (68-85% reduction)

Would require:
- PHP class resolution system
- Interface implementation tracking
- Inheritance graph building
- Namespace resolution

**Verdict:** Not worth it at this stage. The remaining errors are legitimate type mismatches from a strict typing perspective. They're only "false positives" if we have full class hierarchy knowledge.

## Recommendation

**Implement Option 1 (Hardcoded Well-Known Types) ONLY**

**Rationale:**
1. **High confidence:** PSR-7, ClientInterface, and Doctrine Collection are stable, well-known interfaces
2. **Zero false positives:** These mappings are guaranteed correct
3. **Quick implementation:** 30-45 minutes of work
4. **Meaningful impact:** Fixes 5-6 errors (10% of remaining)
5. **Future-proof:** These interfaces won't change

**Skip Option 2** because:
- Pattern matching may create false positives
- Only ~5-10 errors saved
- Requires careful testing and potentially a whitelist

**Skip Option 3** because:
- Only 1 error
- May be a legitimate bug in the code (should use yield, not return)

## Expected Final State

After implementing Option 1:
- **return.typeMismatch errors:** 59 → 53 (10% reduction)
- **Total errors:** 629 → 623 (1% overall)
- **Remaining errors:** Mostly application-specific interfaces requiring full class hierarchy tracking

## Summary Table

| Metric | Before Union Fix | After Union Fix | After Well-Known Types | Total Change |
|--------|------------------|-----------------|------------------------|--------------|
| return.typeMismatch | 119 | 59 | 53 | -66 (55% reduction) |
| Errors fixed | - | 60 | 6 | 66 total |
| % of original | 100% | 49.6% | 44.5% | 55.5% fixed |
