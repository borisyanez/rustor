# Analysis of Remaining 59 Return Type Mismatch Errors

## Error Count Tracking

**IMPORTANT: Using phpstan.neon.dist configuration**

### Initial State (before any fixes)
Need to establish baseline - will check git history

### After Union Type + Closure/Callable Fix
- **Total errors with phpstan.neon.dist:** 629 errors
  - 444 variable.possiblyUndefined
  - **59 return.typeMismatch** ⬅️ Current focus
  - 71 property.typeMismatch
  - 33 function.resultUnused
  - 12 void.pure
  - 8 instanceof.alwaysFalse
  - 2 classConstant.notFound

## Categorization of 59 Remaining Errors

### 1. Interface/Implementation Returns: 54 errors (92%)

**Pattern:** Function declares interface return type, but returns concrete implementation

#### Subcat 1a: Application Interfaces (38 errors)
Interfaces defined in the application code:

```
CartVerificationDtoInterface → ClaroDto, KioskDto, TigoDto, KioskPeDto (4 errors)
CartVerificationValidatorInterface → ClaroValidator, KioskValidator, TigoValidator, KioskPeValidator (4 errors)
IPaymentHistoryQuery → PaymentHistory (1 error)
EarlyPayoffCalculationInterface → NullEarlyPayoffCalculation (1 error)
FeeCalculationInterface → NullFeeCalculation (1 error)
FeeSchemaGeneratorInterface → FeeSchemaGenerator (1 error)
PespayServiceInterface → PespayDownPaymentServiceStub, PespayDownPaymentService (2 errors)
NotifiesDeviceFinanceStatus → NullPartnerApi (1 error)
EntityActionContract → LogInformationalAlert, Deny (2 errors)
ExecutableActionInterface → CreditLineSendNotification, CreditLineSetAccountStatus, CreditLineSetWithdrawalStatus (3 errors)
IdsApiGetOffersServiceInterface → IdsApiGetOffersService (1 error)
FetchOffersStrategyInterface → DefaultOffersStrategy, IdsapiOffersStrategy (2 errors)
TicketExpirationServiceInterface → TicketExpirationService (1 error)
TicketingServiceInterface → TicketingService (1 error)
EarlyPayoffFixerInterface → PeEarlyPayoffFixer, CoEarlyPayoffFixer (2 errors)
OpenpayServiceInterface → OpenpayCustomerPaymentServiceStub, OpenpayDownPaymentServiceStub, OpenpayDownPaymentService (3 errors)
PaymentHandlerInterface → InStorePaymentHandler, SubscriptionPaymentHandler, AsaasPaymentHandler, FlashPaymentHandler, NoPaymentReferenceHandler, PaymentReferenceExistHandler (6 errors)
DebitScheduleServiceInterface → FutureDebitScheduleService, CurrentDebitScheduleService (2 errors)
```

#### Subcat 1b: PSR/Standard Interfaces (4 errors)
Well-known PHP-FIG interfaces:

```
ResponseInterface → Response (3 errors) - PSR-7
ClientInterface → Client (1 error) - HTTP client
```

#### Subcat 1c: Doctrine/ORM Interfaces (1 error)
```
Collection → ArrayCollection (1 error) - Doctrine Collections
```

#### Subcat 1d: Parent Class Returns (11 errors)
Returning subclass when parent class expected:

```
DebtAcknowledgement → ZADebtAcknowledgement (1 error)
(Various transformers and services - 10 more)
```

### 2. Variable Class Returns: 2 errors (3%)

**Pattern:** Returning a variable containing a class name

```
?PaymentMinimumCalculator but returns $calculatorClass (1 error)
Line 21: Function returns variable class name instead of known type
```

### 3. Generator Returns: 1 error (2%)

**Pattern:** Declaring Generator but returning array

```
Generator but returns array (1 error)
```

**Analysis:** This could be:
- A bug (should yield, not return)
- Or Rustor not recognizing a generator function

### 4. Truncated/Incomplete Messages: 2 errors (3%)

Lines with incomplete error messages that need investigation:
```
Line 12: SimpleIrrConfigValidator. (truncated)
Line 23: but returns MxTaxRuleService. (missing expected type)
Line 28: AlsoCorpPaymentInformation. (truncated)
Line 34: PhoneFinanceCreditLineTransformer. (truncated)
Line 35: PhoneFinanceAprFeesTransformer. (truncated)
Line 36: PhoneFinanceMultiplesWithVariantsOffersApiTransformer. (truncated)
Line 37: PhoneFinanceMultiplesOffersApiTransformer. (truncated)
Line 41: but returns Response. (missing expected type)
Line 42: returns RuntimeException. (missing expected type)
Line 45: returns LocalElectronicDefaultSoapClient. (missing expected type)
Line 46: DefaultIrrScheduleEntryCreator. (truncated)
Line 49: OpenpayCustomerPaymentServiceStub. (truncated)
Line 52: int. (truncated)
Line 59: \BrazilOnlineDownPaymentApiClient. (truncated)
```

## Fixable Patterns

### Quick Wins (Low Effort)

#### 1. PSR-7 Interface Compatibility (3-4 errors) ✅ EASY
Hardcode well-known PSR-7 interfaces:
```rust
// ResponseInterface is implemented by Response
if expected.ends_with("responseinterface") && actual == "response" {
    return true;
}

// RequestInterface is implemented by Request
if expected.ends_with("requestinterface") && actual == "request" {
    return true;
}

// UriInterface is implemented by Uri
if expected.ends_with("uriinterface") && actual == "uri" {
    return true;
}

// StreamInterface is implemented by Stream
if expected.ends_with("streaminterface") && actual == "stream" {
    return true;
}
```

**Impact:** 3-4 errors fixed

#### 2. Doctrine Collections (1 error) ✅ EASY
```rust
// Collection is implemented by ArrayCollection
if expected == "collection" && actual == "arraycollection" {
    return true;
}
```

**Impact:** 1 error fixed

#### 3. Common Naming Patterns (5-10 errors) ⚠️ MODERATE
Recognize pattern where concrete class name is interface name without "Interface" suffix:

```rust
// ServiceInterface → Service, etc.
if expected.ends_with("interface") {
    let base = &expected[..expected.len() - 9]; // remove "interface"
    if actual == base {
        return true;
    }
}
```

**Examples this would fix:**
- TicketingServiceInterface → TicketingService ✓
- TicketExpirationServiceInterface → TicketExpirationService ✓
- FeeSchemaGeneratorInterface → FeeSchemaGenerator ✓

**Impact:** ~5-10 errors fixed
**Risk:** May create false positives if pattern doesn't always hold

### Medium Effort

#### 4. Generator Detection (1 error) ⚠️ MODERATE
Check if function contains `yield` statements:
- Already have `block_contains_yield()` method in return_type.rs
- Could use this to infer function returns Generator

**Impact:** 1 error fixed

### High Effort (Not Recommended Now)

#### 5. Full Interface Tracking (40+ errors) ❌ COMPLEX
Would require:
- PHP class resolution system
- Interface implementation tracking
- Inheritance graph building
- Namespace resolution

**Impact:** 40+ errors fixed
**Effort:** Multiple days of work

#### 6. Variable Class Resolution (2 errors) ❌ COMPLEX
Would require tracking variable assignments and types

## Recommended Implementation Plan

### Phase 1: Hardcoded Well-Known Types (Quick Win)
Implement fixes for:
1. PSR-7 interfaces (3-4 errors)
2. Doctrine Collection (1 error)
3. HTTP ClientInterface (1 error)

**Total: ~5-6 errors fixed (8-10% reduction)**
**Effort:** 30 minutes
**Risk:** Very low - these are well-known, stable interfaces

### Phase 2: Naming Pattern Recognition (Optional)
Implement Interface suffix pattern matching

**Total: ~5-10 additional errors fixed (8-17% reduction)**
**Effort:** 1 hour
**Risk:** Low-moderate - may need allow-list to prevent false positives

### Phase 3: Generator Detection (Optional)
Use existing yield detection

**Total: 1 additional error fixed**
**Effort:** 30 minutes
**Risk:** Low

## Total Potential Quick Wins
- **Conservative:** 5-6 errors (8-10% of remaining)
- **With patterns:** 11-17 errors (19-29% of remaining)
- **With generator:** 12-18 errors (20-31% of remaining)

## Remaining Errors After Quick Wins
After implementing all quick wins: **41-47 errors remaining (70-80% of current)**

These would all require full class hierarchy tracking to fix properly.

## Next Steps

1. Implement Phase 1 (PSR-7 + Doctrine) - highest value, lowest risk
2. Test on real codebase
3. Evaluate if Phase 2 pattern matching is worth the risk
4. Document that remaining ~40 errors require architectural work (class hierarchy tracking)
