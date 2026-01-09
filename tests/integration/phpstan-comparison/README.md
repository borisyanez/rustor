# PHPStan Compatibility Testing

This directory contains integration tests that compare rustor-analyze output with PHPStan to ensure compatibility.

## Running Tests

```bash
# Run all comparisons at level 0
./run_comparison.sh 0

# Run all comparisons at level 2
./run_comparison.sh 2

# Run a specific fixture
./run_comparison.sh 2 level0_undefined_function.php
```

## Current Gap Analysis

### Summary Table

| Check | PHPStan ID | Level | Rustor Status |
|-------|-----------|-------|---------------|
| Undefined function | `function.notFound` | 0 | ✅ Implemented |
| Undefined class | `class.notFound` | 0 | ✅ Implemented |
| Undefined variable | `variable.undefined` | 1 | ⚠️ Partial (misses conditional) |
| Undefined method | `method.notFound` | 2 | ✅ Implemented |
| Undefined static method | `staticMethod.notFound` | 0 | ✅ Implemented |
| Undefined property | `property.notFound` | 2 | ✅ Implemented |
| Undefined class constant | `classConstant.notFound` | 0 | ✅ Implemented |
| Argument count | `arguments.count` | 0 | ✅ Implemented (functions only) |

### Detailed Findings

#### Level 0 Checks

1. **Undefined Function** ✅
   - PHPStan: `Function undefined_function not found.`
   - Rustor: `Call to undefined function undefined_function()`
   - Status: **MATCH**

2. **Undefined Class** ✅
   - PHPStan: `Instantiated class UndefinedClass not found.`
   - Rustor: `Class UndefinedClass not found`
   - Status: **MATCH**

3. **Undefined Static Method** ✅
   - PHPStan: `Call to an undefined static method TestClass::undefinedStatic()`
   - Rustor: `Call to an undefined static method TestClass::undefinedStatic().`
   - Status: **MATCH**

4. **Undefined Class Constant** ✅
   - PHPStan: `Access to undefined constant TestClass::UNDEFINED_CONST`
   - Rustor: `Access to undefined constant TestClass::UNDEFINED_CONST.`
   - Status: **MATCH**

5. **Argument Count** ✅
   - PHPStan: `Function requiresTwo invoked with 1 parameter, 2 required.`
   - Rustor: `Function requiresTwo invoked with 1 parameter, 2 required.`
   - Status: **MATCH** (functions only, method argument count not yet implemented)

#### Level 1 Checks

1. **Undefined Variable** ⚠️
   - Simple case: Rustor finds `$undefined`
   - Conditional case: Rustor misses `$conditionalVar` (defined only in if branch)
   - Status: **PARTIAL** - Control flow analysis needed

#### Level 2 Checks

1. **Undefined Method** ✅
   - PHPStan: `Call to an undefined method TestClass::undefinedMethod()`
   - Rustor: `Call to an undefined method TestClass::undefinedMethod().`
   - Status: **MATCH** (requires type tracking through variable assignments)

2. **Undefined Property** ✅
   - PHPStan: `Access to an undefined property TestClass::$undefinedProp`
   - Rustor: `Access to an undefined property TestClass::$undefinedProp.`
   - Status: **MATCH** (requires type tracking through variable assignments)

## Implementation Status

### Completed ✅
- Symbol table integration for class methods, properties, and constants
- `CallStaticMethodsCheck` using symbol table
- `ClassConstantCheck` using symbol table
- `ArgumentCountCheck` for functions
- Type tracking for variable assignments (`$obj = new TestClass()`)
- `CallMethodsCheck` with type-aware checking
- `PropertyAccessCheck` with type-aware checking

### Remaining Work
1. **Control Flow Analysis** - Track variable definitions through branches for possibly-undefined variables
2. **Method Argument Count** - Validate argument counts for method calls
3. **Constructor Argument Count** - Validate argument counts for `new ClassName()` calls

## Test Fixtures

| File | Tests |
|------|-------|
| `level0_undefined_function.php` | Undefined function calls |
| `level0_undefined_class.php` | Undefined class instantiation, extends |
| `level0_undefined_method.php` | Undefined instance method calls |
| `level0_undefined_static_method.php` | Undefined static method calls |
| `level0_undefined_property.php` | Undefined property access |
| `level0_undefined_constant.php` | Undefined class constant access |
| `level1_undefined_variable.php` | Undefined and possibly-undefined variables |
| `level2_argument_count.php` | Wrong argument counts |

## PHPStan Level Reference

| Level | Description |
|-------|-------------|
| 0 | Basic checks: undefined functions, classes, static methods, constants, argument counts |
| 1 | Possibly undefined variables, unknown magic methods |
| 2 | Unknown methods on known types, verify PHPDocs |
| 3 | Return types, property types |
| 4 | Basic dead code checking |
| 5 | Argument types |
| 6 | Report missing typehints |
| 7 | Report partially wrong union types |
| 8 | Report nullable issues |
| 9 | Strict mixed type checking |
