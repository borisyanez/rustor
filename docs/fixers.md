# Fixers Reference

Rustor includes 33 PHP-CS-Fixer compatible fixers for code formatting. These fixers enforce PSR-12 coding standards and can be run separately from refactoring rules.

## Quick Start

```bash
# Run fixers in check mode (dry-run)
rustor src/ --fixer

# Apply fixes
rustor src/ --fixer --fix

# Run specific fixer
rustor src/ --fixer --rule no_trailing_whitespace

# Use PSR-12 preset
rustor src/ --fixer --preset psr12
```

## Configuration

### CLI Options

| Option | Description |
|--------|-------------|
| `--fixer` | Run formatting fixers only (no refactoring rules) |
| `--fixer-config FILE` | Load PHP-CS-Fixer config file |
| `--preset psr12` | Use PSR-12 fixer preset |

### Configuration File

Add fixer settings to `.rustor.toml`:

```toml
[fixer]
preset = "psr12"

[fixer.whitespace]
indent = "spaces"      # "spaces" or "tabs"
indent_size = 4
line_ending = "lf"     # "lf" or "crlf"
```

---

## Whitespace Fixers

### encoding

Ensures files use UTF-8 encoding without BOM.

**PHP-CS-Fixer name:** `encoding`
**Priority:** 100

```php
// Before: File with BOM
<?php // Has invisible BOM bytes

// After: Clean UTF-8
<?php // No BOM
```

### full_opening_tag

Ensures PHP opening tag is `<?php` (not short tag `<?`).

**PHP-CS-Fixer name:** `full_opening_tag`
**Priority:** 90

```php
// Before
<? echo "hello";

// After
<?php echo "hello";
```

### blank_line_after_opening_tag

Ensures a blank line after the PHP opening tag.

**PHP-CS-Fixer name:** `blank_line_after_opening_tag`
**Priority:** 70

```php
// Before
<?php
namespace App;

// After
<?php

namespace App;
```

### line_ending

Normalizes line endings to LF (Unix-style) or CRLF (Windows-style).

**PHP-CS-Fixer name:** `line_ending`
**Priority:** 70

### no_trailing_whitespace

Removes trailing whitespace from lines.

**PHP-CS-Fixer name:** `no_trailing_whitespace`
**Priority:** 70

```php
// Before
$a = 1;   ␣␣␣

// After
$a = 1;
```

### no_whitespace_in_blank_line

Removes whitespace from blank lines.

**PHP-CS-Fixer name:** `no_whitespace_in_blank_line`
**Priority:** 70

```php
// Before
$a = 1;
    ␣␣␣␣
$b = 2;

// After
$a = 1;

$b = 2;
```

### indentation

Normalizes indentation (spaces or tabs).

**PHP-CS-Fixer name:** `indentation_type`
**Priority:** 50

```php
// Before (mixed)
function foo() {
→   if (true) {
        bar();
→   }
}

// After (spaces)
function foo() {
    if (true) {
        bar();
    }
}
```

### single_blank_line_at_eof

Ensures exactly one blank line at end of file.

**PHP-CS-Fixer name:** `single_blank_line_at_end_of_file`
**Priority:** 70

---

## Casing Fixers

### lowercase_keywords

Converts PHP keywords to lowercase.

**PHP-CS-Fixer name:** `lowercase_keywords`
**Priority:** 40

```php
// Before
IF ($a) {
    RETURN TRUE;
} ELSE {
    RETURN FALSE;
}

// After
if ($a) {
    return true;
} else {
    return false;
}
```

### constant_case

Ensures `true`, `false`, and `null` are lowercase.

**PHP-CS-Fixer name:** `constant_case`
**Priority:** 40

```php
// Before
$a = TRUE;
$b = FALSE;
$c = NULL;

// After
$a = true;
$b = false;
$c = null;
```

### lowercase_static_reference

Converts `self`, `static`, and `parent` to lowercase.

**PHP-CS-Fixer name:** `lowercase_static_reference`
**Priority:** 40

```php
// Before
class Foo {
    public function bar() {
        return SELF::class;
    }
}

// After
class Foo {
    public function bar() {
        return self::class;
    }
}
```

---

## Braces & Control Structure Fixers

### braces_position

Controls opening brace placement (PSR-12 style).

**PHP-CS-Fixer name:** `braces_position`
**Priority:** 35

- Classes, interfaces, traits: brace on next line
- Functions/methods: brace on next line
- Control structures: brace on same line

```php
// Before
class Foo {
    public function bar()
    {
        if ($a)
        {
            // ...
        }
    }
}

// After (PSR-12)
class Foo
{
    public function bar()
    {
        if ($a) {
            // ...
        }
    }
}
```

### elseif

Converts `else if` to `elseif`.

**PHP-CS-Fixer name:** `elseif`
**Priority:** 30

```php
// Before
if ($a) {
} else if ($b) {
}

// After
if ($a) {
} elseif ($b) {
}
```

### switch_case_space

Fixes spacing in switch case/default statements.

**PHP-CS-Fixer name:** `switch_case_space`
**Priority:** 30

```php
// Before
switch ($a) {
    case  1  :
    default  :
}

// After
switch ($a) {
    case 1:
    default:
}
```

### no_closing_tag

Removes closing `?>` tag from files.

**PHP-CS-Fixer name:** `no_closing_tag`
**Priority:** 80

```php
// Before
<?php
$a = 1;
?>

// After
<?php
$a = 1;
```

---

## Function Fixers

### function_declaration

Fixes spacing in function declarations.

**PHP-CS-Fixer name:** `function_declaration`
**Priority:** 30

```php
// Before
function  foo () {}
function foo( $a ) {}

// After
function foo() {}
function foo($a) {}
```

### method_argument_space

Fixes spacing in method/function arguments.

**PHP-CS-Fixer name:** `method_argument_space`
**Priority:** 20

```php
// Before
foo( $a,$b , $c );
foo($a,  $b);

// After
foo($a, $b, $c);
foo($a, $b);
```

### return_type_declaration

Fixes return type declaration spacing (PSR-12).

**PHP-CS-Fixer name:** `return_type_declaration`
**Priority:** 30

```php
// Before
function foo() : int {}
function bar():int {}

// After
function foo(): int {}
function bar(): int {}
```

---

## Operator Fixers

### binary_operator_spaces

Ensures single space around binary operators.

**PHP-CS-Fixer name:** `binary_operator_spaces`
**Priority:** 20

```php
// Before
$a=$b+$c;
$a  =  $b;

// After
$a = $b + $c;
$a = $b;
```

### concat_space

Fixes spacing around concatenation operator.

**PHP-CS-Fixer name:** `concat_space`
**Priority:** 20

```php
// Before (no space)
$a = 'Hello'.'World';

// After (with space - configurable)
$a = 'Hello' . 'World';
```

### unary_operator_spaces

Removes space after unary operators (`!`, `~`, `++`, `--`).

**PHP-CS-Fixer name:** `unary_operator_spaces`
**Priority:** 20

```php
// Before
if (! $a) {}
$b = - 5;

// After
if (!$a) {}
$b = -5;
```

---

## Import/Namespace Fixers

### blank_line_after_namespace

Ensures blank line after namespace declaration.

**PHP-CS-Fixer name:** `blank_line_after_namespace`
**Priority:** 20

```php
// Before
namespace App;
use Foo;

// After
namespace App;

use Foo;
```

### no_leading_import_slash

Removes leading backslash from imports.

**PHP-CS-Fixer name:** `no_leading_import_slash`
**Priority:** 20

```php
// Before
use \App\Model;

// After
use App\Model;
```

### single_line_after_imports

Ensures blank line after use statements.

**PHP-CS-Fixer name:** `single_line_after_imports`
**Priority:** 20

```php
// Before
use App\Model;
class Foo {}

// After
use App\Model;

class Foo {}
```

### ordered_imports

Sorts use statements alphabetically and groups by type.

**PHP-CS-Fixer name:** `ordered_imports`
**Priority:** 20

```php
// Before
use function strlen;
use App\Zebra;
use const PHP_VERSION;
use App\Alpha;

// After
use App\Alpha;
use App\Zebra;
use function strlen;
use const PHP_VERSION;
```

### single_import_per_statement

Splits grouped imports into separate statements.

**PHP-CS-Fixer name:** `single_import_per_statement`
**Priority:** 20

```php
// Before
use App\{Model, Controller, View};

// After
use App\Model;
use App\Controller;
use App\View;
```

### no_unused_imports

Removes unused use statements. **Risky fixer.**

**PHP-CS-Fixer name:** `no_unused_imports`
**Priority:** 10
**Risky:** Yes

```php
// Before
use App\Model;  // Not used anywhere
use App\User;   // Used in code

class Foo {
    public function bar(User $user) {}
}

// After
use App\User;

class Foo {
    public function bar(User $user) {}
}
```

---

## Comment Fixers

### no_trailing_whitespace_in_comment

Removes trailing whitespace from comments.

**PHP-CS-Fixer name:** `no_trailing_whitespace_in_comment`
**Priority:** 30

```php
// Before
// Comment with trailing spaces   ␣␣
/* Another comment ␣*/

// After
// Comment with trailing spaces
/* Another comment */
```

### single_line_comment_style

Converts `#` comments to `//` style.

**PHP-CS-Fixer name:** `single_line_comment_style`
**Priority:** 30

```php
// Before
# This is a comment

// After
// This is a comment
```

Note: PHP 8 attributes (`#[Attribute]`) are preserved.

### multiline_whitespace_before_semicolons

Removes whitespace before semicolons.

**PHP-CS-Fixer name:** `multiline_whitespace_before_semicolons`
**Priority:** 20

```php
// Before
$a = 1 ;
$b = foo() ;

// After
$a = 1;
$b = foo();
```

---

## Class/Visibility Fixers

### visibility_required

Ensures visibility modifiers on class members.

**PHP-CS-Fixer name:** `visibility_required`
**Priority:** 30

```php
// Before
class Foo {
    var $a;
    function bar() {}
}

// After
class Foo {
    public $a;
    public function bar() {}
}
```

### no_blank_lines_after_class_opening

Removes blank lines after class opening brace.

**PHP-CS-Fixer name:** `no_blank_lines_after_class_opening`
**Priority:** 30

```php
// Before
class Foo {

    public $a;
}

// After
class Foo {
    public $a;
}
```

### class_definition

Fixes spacing in class/interface/trait definitions.

**PHP-CS-Fixer name:** `class_definition`
**Priority:** 30

```php
// Before
class  Foo  extends  Bar  implements  Baz {}
class Foo{}

// After
class Foo extends Bar implements Baz {}
class Foo {}
```

---

## Fixer Execution Order

Fixers run in priority order (highest first):

1. **100**: encoding
2. **90**: full_opening_tag
3. **80**: no_closing_tag
4. **70**: line_ending, trailing_whitespace, blank lines
5. **50**: indentation
6. **40**: casing fixers
7. **35**: braces_position
8. **30**: function_declaration, return_type, visibility, comments
9. **20**: operators, imports, method_argument_space
10. **10**: no_unused_imports (risky, runs last)

## PSR-12 Preset

The `psr12` preset includes all fixers necessary for PSR-12 compliance:

```bash
rustor src/ --fixer --preset psr12
```

Enabled fixers:
- All whitespace fixers
- All casing fixers
- All braces fixers
- All function fixers
- All operator fixers
- All import fixers (except `no_unused_imports`)
- All comment fixers
- All visibility fixers

## Risky Fixers

Some fixers are marked as "risky" because they may change code behavior:

| Fixer | Risk |
|-------|------|
| `no_unused_imports` | May remove imports used only in docblocks or strings |

Risky fixers are not included in default presets. Enable them explicitly:

```bash
rustor src/ --fixer --rule no_unused_imports
```

Or in configuration:

```toml
[fixer.rules]
no_unused_imports = true
risky_allowed = true
```
