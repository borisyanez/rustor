# YAML Rules Design for Rustor

## Goals
1. **100% Rector rule coverage** - Express any Rector rule in YAML
2. **Easy authoring** - Non-Rust developers can write rules
3. **Dual execution** - Interpret at runtime OR compile to Rust
4. **Test-driven** - Inline test cases in rule definitions

## Rule Schema

### Basic Structure

```yaml
# Rule metadata
name: is_null_to_comparison
description: Replace is_null($x) with $x === null
category: code_quality
min_php: 7.0
max_php: ~  # optional upper bound

# What to match
match:
  node: FuncCall
  name: is_null
  args:
    - capture: $expr

# What to produce
replace: "$expr === null"

# Or for complex replacements:
replace:
  node: BinaryOp
  operator: "==="
  left: $expr
  right:
    node: Null

# Test cases (required)
tests:
  - input: "is_null($x)"
    output: "$x === null"
  - input: "is_null($obj->prop)"
    output: "$obj->prop === null"
  - input: "!is_null($x)"
    output: "$x !== null"
    # Note: negation handling is automatic
```

### Pattern Matching DSL

```yaml
# 1. Function call patterns
match:
  node: FuncCall
  name: old_function
  args:
    - capture: $first
    - capture: $second
    - literal: true  # match specific value

# 2. Method call patterns
match:
  node: MethodCall
  object: capture: $obj
  method: getName
  args: []

# 3. Static method call patterns
match:
  node: StaticCall
  class: SomeClass
  method: staticMethod
  args:
    - capture: $arg

# 4. Binary operation patterns
match:
  node: BinaryOp
  operator: "!=="
  left:
    node: FuncCall
    name: strpos
    args:
      - capture: $haystack
      - capture: $needle
  right:
    node: LiteralFalse

# 5. Ternary patterns
match:
  node: Ternary
  condition:
    node: FuncCall
    name: isset
    args:
      - capture: $var
  then: $var  # same as captured
  else: capture: $default

# 6. Array syntax patterns
match:
  node: Array
  syntax: long  # array() vs []
  items:
    - capture: $items...  # spread capture

# 7. Compound patterns (AND)
match:
  all:
    - node: FuncCall
      name: substr
      args:
        - capture: $str
        - literal: 0
        - capture: $len
    - $len:
        node: FuncCall
        name: strlen
        args:
          - capture: $needle
```

### Replacement DSL

```yaml
# 1. Simple string template
replace: "$expr === null"

# 2. Structured replacement
replace:
  node: FuncCall
  name: str_starts_with
  args:
    - $haystack
    - $needle

# 3. Conditional replacement
replace:
  if:
    condition: "$len.value > 0"
    then: "substr($str, 0, $len)"
    else: "$str"

# 4. Multiple replacements (node expansion)
replace:
  multiple:
    - "$stmt1;"
    - "$stmt2;"

# 5. Remove node
replace: remove

# 6. Wrap in expression
replace:
  wrap:
    node: BooleanNot
    expr: $original
```

### Conditions and Guards

```yaml
# Only apply rule when conditions are met
when:
  # Type conditions (requires type analysis)
  - $expr.type: string
  - $obj.type: instanceof(SomeClass)

  # Value conditions
  - $count.value: "> 0"
  - $name.value: matches(/^get/)

  # Structural conditions
  - $expr: not_null
  - $args.count: 2

  # Context conditions
  - in_class: true
  - in_function: true
  - has_parent: IfStatement
```

### Complex Rule Examples

#### Example 1: Ternary to Null Coalesce
```yaml
name: ternary_to_null_coalesce
description: Convert isset($x) ? $x : $default to $x ?? $default
min_php: 7.0

match:
  node: Ternary
  condition:
    node: FuncCall
    name: isset
    args:
      - capture: $var
  then:
    same_as: $var
  else:
    capture: $default

replace: "$var ?? $default"

tests:
  - input: "isset($x) ? $x : 'default'"
    output: "$x ?? 'default'"
  - input: "isset($arr['key']) ? $arr['key'] : null"
    output: "$arr['key'] ?? null"
```

#### Example 2: strpos to str_contains
```yaml
name: strpos_to_str_contains
description: Convert strpos() !== false to str_contains()
min_php: 8.0

match:
  any:
    - node: BinaryOp
      operator: "!=="
      left:
        node: FuncCall
        name: strpos
        args:
          - capture: $haystack
          - capture: $needle
      right:
        node: LiteralFalse
    - node: BinaryOp
      operator: "!="
      left:
        node: FuncCall
        name: strpos
        args:
          - capture: $haystack
          - capture: $needle
      right:
        node: LiteralFalse

replace: "str_contains($haystack, $needle)"

tests:
  - input: "strpos($str, 'needle') !== false"
    output: "str_contains($str, 'needle')"
  - input: "strpos($h, $n) != false"
    output: "str_contains($h, $n)"
```

#### Example 3: Array Push to Assignment
```yaml
name: array_push_to_assignment
description: Convert array_push($arr, $val) to $arr[] = $val
category: code_quality

match:
  node: FuncCall
  name: array_push
  args:
    - capture: $array
    - capture: $value
    - no_more: true  # exactly 2 args

replace: "$array[] = $value"

tests:
  - input: "array_push($arr, $item)"
    output: "$arr[] = $item"
  - input: "array_push($arr, 1, 2, 3)"
    skip: true  # multiple values not supported
```

#### Example 4: Configurable Rule
```yaml
name: rename_function
description: Rename function calls
category: naming
configurable: true

config:
  mappings:
    type: map<string, string>
    description: Function name mappings (old -> new)
    example:
      sizeof: count
      join: implode

match:
  node: FuncCall
  name: $config.mappings.keys
  args:
    capture: $args...

replace:
  node: FuncCall
  name: $config.mappings[$matched_name]
  args: $args

tests:
  config:
    mappings:
      sizeof: count
  cases:
    - input: "sizeof($arr)"
      output: "count($arr)"
```

## Rule Categories

```yaml
# categories.yaml
categories:
  code_quality:
    description: Improve code quality and readability
    icon: star

  dead_code:
    description: Remove unused code
    icon: trash

  php70:
    description: PHP 7.0 modernization
    min_php: 7.0

  php80:
    description: PHP 8.0 modernization
    min_php: 8.0

  php81:
    description: PHP 8.1 modernization
    min_php: 8.1

  naming:
    description: Naming conventions

  type_declaration:
    description: Add type declarations
    requires_types: true  # needs PHPStan integration
```

## Presets (Rule Sets)

```yaml
# presets/modernize-php80.yaml
name: modernize-php80
description: Modernize code to PHP 8.0
min_php: 8.0

rules:
  - ternary_to_null_coalesce
  - strpos_to_str_contains
  - match_expression
  - named_arguments
  - constructor_promotion

exclude:
  - legacy_function_rename  # skip this one
```

## Implementation Phases

### Phase 1: Core YAML Parser
- Parse YAML rule definitions
- Validate against schema
- Build internal Rule IR

### Phase 2: Pattern Matcher
- AST pattern matching engine
- Capture variables from matched nodes
- Support all match types

### Phase 3: Replacer Engine
- Template-based replacement
- Structured node building
- Handle edge cases (comments, formatting)

### Phase 4: Runtime Interpreter
- Execute rules without compilation
- Fast iteration for rule development
- Built-in test runner

### Phase 5: Rust Code Generator
- Generate optimized Rust code
- Compile rules for production
- Benchmark vs interpreted

### Phase 6: Rector Converter
- Parse Rector PHP rules
- Convert to YAML format
- Handle complex patterns with hints

## Benefits

1. **Accessibility**: PHP developers can write rules without Rust knowledge
2. **Rapid Development**: Test rules instantly without compilation
3. **Portability**: YAML rules can be shared, versioned, contributed
4. **Completeness**: Express any pattern, not limited to predefined templates
5. **Testability**: Inline tests ensure correctness
6. **Performance**: Compile to Rust for production speed
