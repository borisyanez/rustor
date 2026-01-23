# Next Session Plan: WITHOUT Baseline Compatibility

## Current Status
- **WITH baseline:** 100% compatible (api/: 0/0, creditline-consumer/: 7/7)
- **WITHOUT baseline:** ~49% (555/1125 errors)

## Gap Analysis

From PHPStan output WITHOUT baseline, the key missing checks are:

### 1. `generics.notSubtype` - HIGH PRIORITY
PHPStan reports many errors like:
- "Type X in generic type Repository<X> is not subtype of template type T of Y"

**Implementation steps:**
1. Parse `@template` annotations to extract template parameters with bounds
2. Track template parameter constraints (e.g., `@template T of Entity`)
3. When a generic class is used (e.g., `Repository<Foo>`), verify Foo satisfies the template bound
4. Report error if type argument doesn't satisfy constraint

**Files to modify:**
- Create `crates/rustor-analyze/src/checks/level6/generics.rs`
- Update `checks/level6/mod.rs` to export
- Register in `checks/mod.rs`

### 2. `staticMethod.notFound` - MEDIUM PRIORITY
PHPStan detects calls to non-existent static methods.

**Implementation steps:**
1. In `undefined_class.rs` or new file, track static method calls
2. Look up class in symbol table
3. Check if the static method exists on the class
4. Report if method not found

**Files to modify:**
- Create `crates/rustor-analyze/src/checks/level0/undefined_static_method.rs`
- Or extend existing `call_static_methods.rs`

### 3. `parameter.phpDocType` - MEDIUM PRIORITY
PHPStan validates that PHPDoc parameter types match method signature.

**Implementation steps:**
1. Parse PHPDoc @param types
2. Compare with actual parameter type hints
3. Report if PHPDoc type is incompatible with declared type

**Files to modify:**
- Create `crates/rustor-analyze/src/checks/level6/phpdoc_param_type.rs`

### 4. `nullCoalesce.expr` - LOW PRIORITY
PHPStan warns when using `??` on expressions that can't be null.

**Implementation steps:**
1. Track type of left operand in `??` expression
2. If type doesn't accept null, report warning
3. Requires type inference for expressions

**Files to modify:**
- Create `crates/rustor-analyze/src/checks/level4/null_coalesce.rs`

### 5. Improve Type Inference - FOUNDATIONAL
Many checks depend on knowing the type of expressions.

**Current gaps:**
- Method return type inference
- Property type tracking
- Variable type narrowing after conditionals

**Implementation steps:**
1. Enhance `types/inference.rs` to track more expression types
2. Add method return type lookup from symbol table
3. Implement control flow type narrowing

## Suggested Order of Implementation

### Session 1: Generics Check (3-4 hours)
1. [ ] Read PHPStan's generics error messages to understand exact format
2. [ ] Design data structure for template parameters with bounds
3. [ ] Parse @template annotations with bounds (e.g., `@template T of Entity`)
4. [ ] Create `generics.rs` check
5. [ ] Implement validation logic
6. [ ] Add tests
7. [ ] Test on creditline-consumer

### Session 2: Static Method Check (2-3 hours)
1. [ ] Analyze current `call_static_methods.rs`
2. [ ] Add method existence validation
3. [ ] Look up method in symbol table
4. [ ] Handle inherited methods
5. [ ] Test on creditline-consumer

### Session 3: PHPDoc Parameter Type (2-3 hours)
1. [ ] Parse PHPDoc @param types alongside native types
2. [ ] Compare PHPDoc type with native type hint
3. [ ] Report mismatches
4. [ ] Handle nullable and union types
5. [ ] Test on creditline-consumer

### Session 4: Type Inference Improvements (4+ hours)
1. [ ] Enhance method call return type inference
2. [ ] Add property type tracking
3. [ ] Implement basic type narrowing
4. [ ] Update dependent checks to use improved inference

## Quick Wins (Can be done anytime)
- [ ] Add more PHP builtin classes to the list
- [ ] Improve error message formatting to match PHPStan exactly
- [ ] Add more PHPDoc type syntax support (callable signatures, etc.)

## Testing Commands
```bash
# Build
cargo build --release -p rustor-cli

# Test WITH baseline (should be 7 errors)
/Users/borisyv/RustProjects/rustor/target/release/rustor analyze \
  /Users/borisyv/PhpProjects/payjoy_www/creditline-consumer/ \
  -c /Users/borisyv/PhpProjects/payjoy_www/phpstan.neon.dist \
  --level 6 --phpstan-compat

# Test WITHOUT baseline (track progress toward 1125)
/Users/borisyv/RustProjects/rustor/target/release/rustor analyze \
  /Users/borisyv/PhpProjects/payjoy_www/creditline-consumer/ \
  --level 6 --no-config

# Compare with PHPStan
cd /Users/borisyv/PhpProjects/payjoy_www && \
php -d auto_prepend_file= ./libs/vendor/bin/phpstan analyze \
  creditline-consumer/ --level 6 --no-progress --memory-limit=-1
```

## Success Metrics
| Metric | Current | Target |
|--------|---------|--------|
| WITHOUT baseline errors | 555 | ~1070 (95% of 1125) |
| Check coverage | ~60% | 95% |
