# Rules Reference

Rustor includes 44 refactoring rules organized into four categories. Each rule is designed to be safe and produce semantically equivalent code.

## Table of Contents

- [Performance Rules](#performance-rules)
- [Modernization Rules](#modernization-rules)
- [Simplification Rules](#simplification-rules)
- [Compatibility Rules](#compatibility-rules)
- [Imported Rules (from Rector)](#imported-rules-from-rector)
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

## Imported Rules (from Rector)

These rules were auto-generated from the [Rector PHP](https://github.com/rectorphp/rector) project using `rustor-import-rector`. They cover additional modernization and simplification patterns.

### Modernization Rules (Imported)

#### `array_key_exists_on_property`

Change `array_key_exists()` on property to `property_exists()`.

**PHP Version:** 7.4+
**Category:** Modernization

```php
// Before
array_key_exists('key', $object);

// After
property_exists($object, 'key');
```

---

#### `class_on_object`

Change `get_class($object)` to faster `$object::class`.

**PHP Version:** 8.0+
**Category:** Modernization

```php
// Before
$className = get_class($object);

// After
$className = $object::class;
```

**Notes:**
- Similar to `get_class_this` but handles any variable, not just `$this`
- Using `::class` on objects was added in PHP 8.0

---

#### `filter_var_to_add_slashes`

Change `filter_var()` with slash escaping to `addslashes()`.

**PHP Version:** 7.4+
**Category:** Modernization

```php
// Before
$escaped = filter_var($str, FILTER_SANITIZE_MAGIC_QUOTES);

// After
$escaped = addslashes($str);
```

**Notes:**
- `FILTER_SANITIZE_MAGIC_QUOTES` was deprecated in PHP 7.4 and removed in PHP 8.0

---

#### `hebrevc_to_nl_2_br_hebrev`

Change `hebrevc()` to `nl2br(hebrev())`.

**PHP Version:** 7.4+
**Category:** Modernization

```php
// Before
$result = hebrevc($str);

// After
$result = nl2br(hebrev($str));
```

**Notes:**
- `hebrevc()` was deprecated in PHP 7.4 and removed in PHP 8.0

---

#### `remove_get_class_get_parent_class_no_args`

Replace `get_class()` and `get_parent_class()` without arguments.

**PHP Version:** 8.3+
**Category:** Modernization

```php
// Before
$class = get_class();
$parent = get_parent_class();

// After
$class = self::class;
$parent = parent::class;
```

**Notes:**
- Calling these functions without arguments was deprecated in PHP 8.3

---

#### `restore_include_path_to_ini_restore`

Change `restore_include_path()` to `ini_restore('include_path')`.

**PHP Version:** 7.4+
**Category:** Modernization

```php
// Before
restore_include_path();

// After
ini_restore('include_path');
```

**Notes:**
- `restore_include_path()` was deprecated in PHP 7.4 and removed in PHP 8.0

---

#### `utf_8_decode_encode_to_mb_convert_encoding`

Change deprecated `utf8_decode()` and `utf8_encode()` to `mb_convert_encoding()`.

**PHP Version:** 8.2+
**Category:** Modernization

```php
// Before
$decoded = utf8_decode($value);
$encoded = utf8_encode($value);

// After
$decoded = mb_convert_encoding($value, 'ISO-8859-1');
$encoded = mb_convert_encoding($value, 'UTF-8', 'ISO-8859-1');
```

**Notes:**
- `utf8_decode()` and `utf8_encode()` were deprecated in PHP 8.2

---

### Simplification Rules (Imported)

#### `consistent_implode`

Changes various `implode()` forms to consistent argument order.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
$str = implode($array);

// After
$str = implode('', $array);
```

**Notes:**
- Ensures consistent argument order for `implode()` calls

---

#### `get_class_on_null`

Handle `get_class()` behavior change with null arguments.

**PHP Version:** 7.2+
**Category:** Simplification

```php
// Before
$class = get_class($maybeNull);

// After
$class = $maybeNull !== null ? get_class($maybeNull) : self::class;
```

**Notes:**
- In PHP 8.0+, `get_class(null)` throws an error instead of returning the current class

---

#### `inline_is_a_instance_of`

Change `is_a()` with object and class name check to `instanceof`.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
if (is_a($object, SomeType::class)) { }

// After
if ($object instanceof SomeType) { }
```

**Notes:**
- `instanceof` is more readable and idiomatic PHP

---

#### `is_a_with_string_with_third_argument`

Complete missing 3rd argument in `is_a()` function for string class names.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
is_a($className, ParentClass::class);

// After
is_a($className, ParentClass::class, true);
```

**Notes:**
- When first argument is a string (class name), third argument should be `true`

---

#### `pow_to_exp`

Changes `pow()` to `**` operator.

**PHP Version:** 5.6+
**Category:** Simplification

```php
// Before
$result = pow($base, $exp);

// After
$result = $base ** $exp;
```

**Notes:**
- Same as built-in `pow_to_operator` rule, imported from Rector

---

#### `preg_replace_e_modifier`

The `/e` modifier is no longer supported, use `preg_replace_callback` instead.

**PHP Version:** 5.5+
**Category:** Simplification

```php
// Before
preg_replace('/pattern/e', 'strtoupper("$1")', $subject);

// After
preg_replace_callback('/pattern/', function($m) { return strtoupper($m[1]); }, $subject);
```

**Notes:**
- The `/e` modifier was deprecated in PHP 5.5 and removed in PHP 7.0

---

#### `remove_sole_value_sprintf`

Remove `sprintf()` wrapper if not needed.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
$str = sprintf('%s', $value);

// After
$str = $value;
```

**Notes:**
- Removes unnecessary `sprintf()` calls with single `%s` placeholder

---

#### `remove_useless_is_object_check`

Remove useless `is_object()` check combined with `instanceof`.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
if (is_object($var) && $var instanceof SomeClass) { }

// After
if ($var instanceof SomeClass) { }
```

**Notes:**
- `instanceof` already returns false for non-objects

---

#### `rename_mktime_without_args_to_time`

Rename `mktime()` without arguments to `time()`.

**PHP Version:** 7.0+
**Category:** Simplification

```php
// Before
$timestamp = mktime();

// After
$timestamp = time();
```

**Notes:**
- `mktime()` without arguments was deprecated in PHP 5.1

---

#### `simplify_strpos_lower`

Simplify `strpos(strtolower())` patterns.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
strpos(strtolower($haystack), strtolower($needle));

// After
stripos($haystack, $needle);
```

**Notes:**
- Uses case-insensitive `stripos()` instead of lowercasing both strings

---

#### `ternary_implode_to_implode`

Narrow ternary with `implode()` and empty string to direct `implode()`.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
$str = count($arr) > 0 ? implode(',', $arr) : '';

// After
$str = implode(',', $arr);
```

**Notes:**
- `implode()` on empty array returns empty string anyway

---

#### `unwrap_sprintf_one_argument`

Unwrap `sprintf()` with one argument.

**PHP Version:** Any
**Category:** Simplification

```php
// Before
$str = sprintf($template);

// After
$str = $template;
```

**Notes:**
- `sprintf()` with no placeholders just returns the format string

---

## Rule Configuration

### Presets

| Preset | Rules |
|--------|-------|
| `recommended` | array_push, array_syntax, implode_order, is_null, isset_coalesce, sizeof |
| `performance` | array_key_first_last, array_push, pow_to_operator, sizeof, type_cast |
| `modernize` | array_syntax, assign_coalesce, constructor_promotion, first_class_callables, get_class_this, list_short_syntax, isset_coalesce, empty_coalesce, match_expression, null_safe_operator, readonly_properties, string_contains, string_starts_ends |
| `all` | All 44 rules |

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
