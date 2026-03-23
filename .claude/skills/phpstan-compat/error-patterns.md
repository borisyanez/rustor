# PHPStan Error Patterns Reference

Quick reference for PHPStan error identifiers and their rustor equivalents.

## Level 0 Errors

### function.notFound
```
Function foo() not found.
```
- **Rustor check**: `level0::UndefinedFunctionCheck`
- **Common issues**:
  - Function in vendor not in classmap
  - Polyfill functions not recognized
  - Dynamic function calls

### class.notFound
```
Class Foo not found.
```
- **Rustor check**: `level0::UndefinedClassCheck`
- **Common issues**:
  - PSR-4 autoload not scanned
  - Aliased classes
  - Anonymous classes

### constant.notFound
```
Constant FOO not found.
```
- **Rustor check**: `level0::UndefinedConstantCheck`
- **Common issues**:
  - Class constants vs global constants
  - define() vs const

### staticMethod.notFound
```
Call to an undefined static method Foo::bar().
```
- **Rustor check**: `level0::CallStaticMethodsCheck`

### classConstant.notFound
```
Access to undefined constant Foo::BAR.
```
- **Rustor check**: `level0::ClassConstantCheck`

### argument.count
```
Function foo() expects 2 arguments, 1 given.
```
- **Rustor check**: `level0::ArgumentCountCheck`
- **Note**: Requires function signature knowledge

### return.missing
```
Method Foo::bar() should return string but return statement is missing.
```
- **Rustor check**: `level0::MissingReturnCheck`

## Level 1 Errors

### variable.undefined
```
Variable $foo might not be defined.
```
- **Rustor check**: `level1::UndefinedVariableCheck`
- **Note**: Requires control flow analysis

### isset.variable
```
Variable $foo in isset() always exists and is not nullable.
```
- **Rustor check**: `level1::IssetVariableCheck`

## Level 2 Errors

### method.notFound
```
Call to an undefined method Foo::bar().
```
- **Rustor check**: `level2::CallMethodsCheck`
- **Note**: Requires type tracking

### property.notFound
```
Access to an undefined property Foo::$bar.
```
- **Rustor check**: `level2::PropertyAccessCheck`

## Level 3 Errors

### return.type
```
Method Foo::bar() should return int but returns string.
```
- **Rustor check**: `level3::ReturnTypeCheck`

### property.type
```
Property Foo::$bar (int) does not accept string.
```
- **Rustor check**: `level3::PropertyTypeCheck`

## Level 4 Errors

### deadCode.unreachable
```
Unreachable statement - code above always terminates.
```
- **Rustor check**: `level4::DeadCodeCheck`

### booleanAnd.alwaysFalse
```
Result of && is always false.
```
- **Rustor check**: `level4::AlwaysFalseBooleanCheck`

## Level 5 Errors

### argument.type
```
Parameter #1 $x of function foo expects int, string given.
```
- **Rustor check**: `level5::ArgumentTypeCheck`

## Level 6 Errors

### missingType.parameter
```
Method Foo::bar() has parameter $x with no type specified.
```
- **Rustor check**: `level6::MissingTypehintCheck`

## Level 7+ Errors

### union.invalid
```
Result of || is always the same.
```
- **Rustor check**: `level7::UnionTypeCheck`

### nullable.access
```
Cannot call method foo() on null.
```
- **Rustor check**: `level8::NullableAccessCheck`

## Error Message Mapping

When rustor message doesn't match PHPStan exactly:

| PHPStan Message | Rustor Message | Status |
|----------------|----------------|--------|
| `Function foo() not found.` | `undefined function 'foo'` | Needs alignment |
| `Class Foo not found.` | `undefined class 'Foo'` | Needs alignment |

## JSON Output Format

PHPStan JSON format:
```json
{
  "totals": {
    "errors": 0,
    "file_errors": 42
  },
  "files": {
    "/path/to/file.php": {
      "errors": 0,
      "messages": [
        {
          "message": "Function foo() not found.",
          "line": 10,
          "ignorable": true,
          "identifier": "function.notFound"
        }
      ]
    }
  },
  "errors": []
}
```

Rustor should match this format exactly when `--phpstan_compat` is used.

## Symbol Table Requirements

For each check level, required symbol knowledge:

| Level | Required Symbol Knowledge |
|-------|--------------------------|
| 0 | Functions, classes, constants (global) |
| 1 | Variables (local scope) |
| 2 | Methods, properties (class-level) |
| 3 | Return types, property types |
| 4 | Type inference, constant values |
| 5 | Parameter types |
| 6+ | Full type system |

## Autoload Integration

### Classmap
```php
// vendor/composer/autoload_classmap.php
return [
    'App\\Foo' => '/path/to/Foo.php',
];
```
- Parse and load into symbol table

### PSR-4
```json
// composer.json
{
  "autoload": {
    "psr-4": {
      "App\\": "src/"
    }
  }
}
```
- Scan directories, map namespace to files

### Files
```json
{
  "autoload": {
    "files": ["src/helpers.php"]
  }
}
```
- Load function definitions from these files
