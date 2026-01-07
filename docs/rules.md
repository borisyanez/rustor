# Rules Reference

Rustor includes 23 refactoring rules organized into four categories. Each rule is designed to be safe and produce semantically equivalent code.

## Table of Contents

- [Performance Rules](#performance-rules)
- [Modernization Rules](#modernization-rules)
- [Simplification Rules](#simplification-rules)
- [Compatibility Rules](#compatibility-rules)
- [Rule Configuration](#rule-configuration)

---

## Performance Rules

Rules that improve runtime performance.

### `array_push`

Convert `array_push()` with single value to direct array assignment.

**PHP Version:** Any
**Category:** Performance
**Preset:** recommended, performance

```php
// Before
array_push($items, $value);
array_push($data, getResult());

// After
$items[] = $value;
$data[] = getResult();
```

**Notes:**
- Only converts single-value `array_push()` calls
- Skips multi-value calls: `array_push($arr, $a, $b, $c)`
- Skips when return value is used: `$count = array_push($arr, $val)`

---

### `sizeof`

Convert `sizeof()` to `count()`.

**PHP Version:** Any
**Category:** Performance
**Preset:** recommended, performance

```php
// Before
$len = sizeof($array);
if (sizeof($items) > 0) { }

// After
$len = count($array);
if (count($items) > 0) { }
```

**Notes:**
- `sizeof()` is an alias for `count()` but `count()` is the canonical form
- Preserves the second argument if present: `sizeof($arr, COUNT_RECURSIVE)`

---

### `pow_to_operator`

Convert `pow()` function to `**` operator.

**PHP Version:** 5.6+
**Category:** Performance
**Preset:** performance

```php
// Before
$result = pow($base, $exponent);
$square = pow($x, 2);
$cube = pow($n, 3);

// After
$result = $base ** $exponent;
$square = $x ** 2;
$cube = $n ** 3;
```

**Notes:**
- The `**` operator was introduced in PHP 5.6
- Adds parentheses around complex expressions when needed

---

### `type_cast`

Convert `strval()`, `intval()`, `floatval()`, `boolval()` to cast syntax.

**PHP Version:** Any
**Category:** Performance
**Preset:** performance

```php
// Before
$str = strval($number);
$int = intval($input);
$float = floatval($value);
$bool = boolval($flag);

// After
$str = (string) $number;
$int = (int) $input;
$float = (float) $value;
$bool = (bool) $flag;
```

**Notes:**
- Skips `intval()` with base argument: `intval($hex, 16)`
- `doubleval()` is also converted (alias for `floatval()`)

---

### `array_key_first_last`

Convert `array_keys($arr)[0]` patterns to `array_key_first()` / `array_key_last()`.

**PHP Version:** 7.3+
**Category:** Performance
**Preset:** performance

```php
// Before
$first = array_keys($data)[0];
$first = reset(array_keys($data));
$last = end(array_keys($data));

// After
$first = array_key_first($data);
$first = array_key_first($data);
$last = array_key_last($data);
```

**Notes:**
- `array_key_first()` and `array_key_last()` were added in PHP 7.3
- More efficient than creating intermediate array of keys

---

## Modernization Rules

Rules that modernize syntax to newer PHP features.

### `array_syntax`

Convert `array()` to short array syntax `[]`.

**PHP Version:** 5.4+
**Category:** Modernization
**Preset:** recommended, modernize

```php
// Before
$items = array(1, 2, 3);
$map = array('key' => 'value');
$nested = array(array(1, 2), array(3, 4));

// After
$items = [1, 2, 3];
$map = ['key' => 'value'];
$nested = [[1, 2], [3, 4]];
```

**Notes:**
- Recursively converts nested arrays
- Preserves array formatting and comments

---

### `list_short_syntax`

Convert `list()` to short destructuring syntax `[]`.

**PHP Version:** 7.1+
**Category:** Modernization
**Preset:** modernize

```php
// Before
list($a, $b) = $array;
list($first, , $third) = $values;
list('name' => $name, 'age' => $age) = $person;

// After
[$a, $b] = $array;
[$first, , $third] = $values;
['name' => $name, 'age' => $age] = $person;
```

**Notes:**
- Short syntax requires PHP 7.1+
- Keyed list syntax requires PHP 7.1+

---

### `isset_coalesce`

Convert `isset($x) ? $x : $default` to null coalescing operator.

**PHP Version:** 7.0+
**Category:** Modernization
**Preset:** recommended, modernize

```php
// Before
$value = isset($data['key']) ? $data['key'] : 'default';
$name = isset($user->name) ? $user->name : 'Anonymous';

// After
$value = $data['key'] ?? 'default';
$name = $user->name ?? 'Anonymous';
```

**Notes:**
- The `??` operator was introduced in PHP 7.0
- Variable in condition must match variable in true branch exactly

---

### `empty_coalesce`

Convert `empty($x) ? $default : $x` to Elvis operator.

**PHP Version:** 5.3+
**Category:** Modernization
**Preset:** modernize

```php
// Before
$value = empty($input) ? 'default' : $input;
$name = !empty($user) ? $user : 'Guest';

// After
$value = $input ?: 'default';
$name = $user ?: 'Guest';
```

**Notes:**
- The Elvis operator `?:` was introduced in PHP 5.3
- Works with both `empty($x) ? default : $x` and `!empty($x) ? $x : default`

---

### `assign_coalesce`

Convert `$x = $x ?? $default` to null coalescing assignment.

**PHP Version:** 7.4+
**Category:** Modernization
**Preset:** modernize

```php
// Before
$data['key'] = $data['key'] ?? 'default';
$options = $options ?? [];

// After
$data['key'] ??= 'default';
$options ??= [];
```

**Notes:**
- The `??=` operator was introduced in PHP 7.4
- Variable on left must match first operand of `??` exactly

---

### `null_safe_operator`

Convert null checks with method/property access to null-safe operator.

**PHP Version:** 8.0+
**Category:** Modernization
**Preset:** modernize

```php
// Before
$name = $user !== null ? $user->getName() : null;
$city = $address != null ? $address->city : null;

// After
$name = $user?->getName();
$city = $address?->city;
```

**Notes:**
- The `?->` operator was introduced in PHP 8.0
- Only converts when else branch is `null`

---

### `string_contains`

Convert `strpos() !== false` patterns to `str_contains()`.

**PHP Version:** 8.0+
**Category:** Modernization
**Preset:** modernize

```php
// Before
if (strpos($haystack, $needle) !== false) { }
if (strpos($text, 'search') === false) { }

// After
if (str_contains($haystack, $needle)) { }
if (!str_contains($text, 'search')) { }
```

**Configuration:**
```toml
[rules.string_contains]
loose_comparison = false  # Also convert == and != comparisons
```

**Notes:**
- `str_contains()` was added in PHP 8.0
- More readable and semantically clear

---

### `string_starts_ends`

Convert `substr()` comparisons to `str_starts_with()` / `str_ends_with()`.

**PHP Version:** 8.0+
**Category:** Modernization
**Preset:** modernize

```php
// Before
if (substr($path, 0, 1) === '/') { }
if (substr($file, -4) === '.php') { }

// After
if (str_starts_with($path, '/')) { }
if (str_ends_with($file, '.php')) { }
```

**Notes:**
- `str_starts_with()` and `str_ends_with()` were added in PHP 8.0
- Only converts when substring length matches comparison string length

---

### `match_expression`

Convert simple switch statements to match expressions.

**PHP Version:** 8.0+
**Category:** Modernization
**Preset:** modernize

```php
// Before
switch ($status) {
    case 'active': $label = 'Active'; break;
    case 'pending': $label = 'Pending'; break;
    default: $label = 'Unknown';
}

// After
$label = match($status) {
    'active' => 'Active',
    'pending' => 'Pending',
    default => 'Unknown',
};
```

**Requirements for conversion:**
- Each case must have exactly one assignment followed by break
- All cases must assign to the same variable
- At least 2 cases required
- No compound assignments (`+=`, etc.)

---

### `get_class_this`

Convert `get_class($var)` to `$var::class`.

**PHP Version:** 8.0+
**Category:** Modernization
**Preset:** modernize

```php
// Before
$className = get_class($this);
$type = get_class($object);

// After
$className = $this::class;
$type = $object::class;
```

**Notes:**
- Using `::class` on objects was added in PHP 8.0
- More concise and consistent with static `ClassName::class`

---

### `first_class_callables`

Convert `Closure::fromCallable()` to first-class callable syntax.

**PHP Version:** 8.1+
**Category:** Modernization
**Preset:** modernize

```php
// Before
$fn = Closure::fromCallable('strlen');
$fn = Closure::fromCallable([$this, 'method']);
$fn = Closure::fromCallable([self::class, 'staticMethod']);

// After
$fn = strlen(...);
$fn = $this->method(...);
$fn = self::staticMethod(...);
```

**Notes:**
- First-class callable syntax was added in PHP 8.1
- Only converts when argument is a static string or array literal

---

### `constructor_promotion` (Framework)

Convert constructor property assignments to promoted properties.

**PHP Version:** 8.0+
**Category:** Modernization
**Preset:** modernize
**Status:** Detection only (full transformation pending)

```php
// Before
class User {
    private string $name;
    private int $age;

    public function __construct(string $name, int $age) {
        $this->name = $name;
        $this->age = $age;
    }
}

// After
class User {
    public function __construct(
        private string $name,
        private int $age,
    ) {}
}
```

**Notes:**
- Currently detects promotable properties but doesn't transform
- Requires complex multi-span edits (property removal + parameter modification)

---

### `readonly_properties` (Framework)

Add `readonly` modifier to properties only assigned in constructor.

**PHP Version:** 8.1+
**Category:** Modernization
**Preset:** modernize
**Status:** Detection only (full transformation pending)

```php
// Before
class User {
    private string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }
}

// After
class User {
    private readonly string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }
}
```

**Notes:**
- Requires tracking property assignments across all methods
- Currently a framework for future implementation

---

## Simplification Rules

Rules that simplify code.

### `is_null`

Convert `is_null()` to strict comparison.

**PHP Version:** Any
**Category:** Simplification
**Preset:** recommended

```php
// Before
if (is_null($value)) { }
$isNull = is_null($result);
if (!is_null($data)) { }

// After
if ($value === null) { }
$isNull = $result === null;
if ($data !== null) { }
```

**Notes:**
- Strict comparison is slightly faster
- More explicit about type comparison

---

### `join_to_implode`

Convert `join()` to `implode()`.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
$str = join(', ', $array);
$path = join('/', $parts);

// After
$str = implode(', ', $array);
$path = implode('/', $parts);
```

**Notes:**
- `join()` is an alias for `implode()`
- `implode()` is the canonical function name

---

### `sprintf_positional`

Convert simple `sprintf()` with `%s` to string interpolation.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
$msg = sprintf('Hello, %s!', $name);
$log = sprintf('[%s] %s', $level, $message);

// After
$msg = "Hello, {$name}!";
$log = "[{$level}] {$message}";
```

**Notes:**
- Only converts when format uses simple `%s` placeholders
- Skips width specifiers, precision, positional arguments

---

## Compatibility Rules

Rules that ensure compatibility or follow best practices.

### `class_constructor`

Convert legacy PHP 4-style constructors to `__construct`.

**PHP Version:** 7.0+ (deprecated), 8.0+ (removed)
**Category:** Compatibility

```php
// Before
class Foo {
    function Foo($x) {
        $this->x = $x;
    }
}

// After
class Foo {
    function __construct($x) {
        $this->x = $x;
    }
}
```

**Notes:**
- Legacy constructors deprecated in PHP 7.0, removed in PHP 8.0
- Skips if class already has `__construct`
- Skips if method has return type or returns a value

---

### `implode_order`

Fix deprecated `implode()` argument order.

**PHP Version:** 7.4+
**Category:** Compatibility
**Preset:** recommended

```php
// Before (deprecated)
$str = implode($array, ', ');

// After
$str = implode(', ', $array);
```

**Notes:**
- Passing array as first argument deprecated in PHP 7.4, removed in PHP 8.0
- Only converts when first arg is array-like and second is string literal

---

## Rule Configuration

### Presets

| Preset | Rules |
|--------|-------|
| `recommended` | array_push, array_syntax, implode_order, is_null, isset_coalesce, sizeof |
| `performance` | array_key_first_last, array_push, pow_to_operator, sizeof, type_cast |
| `modernize` | array_syntax, assign_coalesce, constructor_promotion, first_class_callables, get_class_this, list_short_syntax, isset_coalesce, empty_coalesce, match_expression, null_safe_operator, readonly_properties, string_contains, string_starts_ends |
| `all` | All 23 rules |

### Per-Rule Configuration

Some rules support configuration options:

```toml
[rules.string_contains]
loose_comparison = true  # Also convert == and != comparisons
```

### Inline Ignore Comments

Disable rules for specific lines:

```php
// rustor-ignore
if (is_null($value)) { }  // Not reported

// rustor-ignore: is_null, isset_coalesce
$x = isset($y) ? $y : $z;  // Not reported

$a = is_null($b);  // rustor-ignore-line

// rustor-ignore-file: sizeof
// Disables sizeof rule for entire file
```
