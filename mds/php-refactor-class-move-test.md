# Test Case: Class Move with Full Reference Update

## Overview

This test validates the most complex refactoring operation: **moving a class to a different namespace/directory and updating ALL references** across the codebase.

This is the "acid test" for the refactoring tool because it requires:
- Cross-file analysis
- Multiple reference types
- PSR-4 namespace mapping
- Handling edge cases (aliased imports, implicit namespace resolution)

---

## Test Scenario

### Before: Original Structure

```
src/
├── Legacy/
│   └── Services/
│       └── PaymentProcessor.php      # CLASS TO MOVE
├── Controllers/
│   └── CheckoutController.php        # Uses PaymentProcessor
├── Services/
│   └── OrderService.php              # Uses PaymentProcessor
├── Models/
│   └── Invoice.php                   # Type hints PaymentProcessor
├── Events/
│   └── PaymentCompleted.php          # PHPDoc reference
├── Tests/
│   └── PaymentProcessorTest.php      # Multiple reference types
├── config/
│   └── services.php                  # String class reference
└── bootstrap.php                     # require_once (legacy)
```

### After: Target Structure

```
src/
├── Legacy/
│   └── Services/
│       └── (empty - file moved)
├── Payment/
│   └── Processing/
│       └── PaymentProcessor.php      # MOVED HERE
├── Controllers/
│   └── CheckoutController.php        # Updated use statement
├── Services/
│   └── OrderService.php              # Updated use statement
├── Models/
│   └── Invoice.php                   # Updated type hints
├── Events/
│   └── PaymentCompleted.php          # Updated PHPDoc
├── Tests/
│   └── PaymentProcessorTest.php      # All references updated
├── config/
│   └── services.php                  # Updated string reference
└── bootstrap.php                     # Updated require path
```

---

## Test Files

### 1. The Class Being Moved

```php
<?php
// src/Legacy/Services/PaymentProcessor.php
// BEFORE

namespace App\Legacy\Services;

use App\Models\Invoice;
use App\Events\PaymentCompleted;

/**
 * Handles payment processing for the application.
 * 
 * @package App\Legacy\Services
 */
class PaymentProcessor
{
    public const STATUS_PENDING = 'pending';
    public const STATUS_COMPLETED = 'completed';
    public const STATUS_FAILED = 'failed';

    private string $apiKey;

    public function __construct(string $apiKey)
    {
        $this->apiKey = $apiKey;
    }

    public function process(Invoice $invoice): PaymentResult
    {
        // Process payment logic
        return new PaymentResult(self::STATUS_COMPLETED);
    }

    public static function create(string $apiKey): self
    {
        return new self($apiKey);
    }

    public static function getDefaultGateway(): string
    {
        return 'stripe';
    }
}
```

```php
<?php
// src/Payment/Processing/PaymentProcessor.php
// AFTER

namespace App\Payment\Processing;  // ← NAMESPACE CHANGED

use App\Models\Invoice;
use App\Events\PaymentCompleted;

/**
 * Handles payment processing for the application.
 * 
 * @package App\Payment\Processing  // ← PHPDOC UPDATED
 */
class PaymentProcessor
{
    // ... rest unchanged
}
```

---

### 2. Simple Use Statement

```php
<?php
// src/Controllers/CheckoutController.php
// BEFORE

namespace App\Controllers;

use App\Legacy\Services\PaymentProcessor;  // ← TO BE UPDATED
use App\Models\Cart;

class CheckoutController
{
    private PaymentProcessor $processor;

    public function __construct(PaymentProcessor $processor)
    {
        $this->processor = $processor;
    }

    public function checkout(Cart $cart): void
    {
        $result = $this->processor->process($cart->toInvoice());
    }
}
```

```php
<?php
// src/Controllers/CheckoutController.php
// AFTER

namespace App\Controllers;

use App\Payment\Processing\PaymentProcessor;  // ← UPDATED
use App\Models\Cart;

class CheckoutController
{
    private PaymentProcessor $processor;

    public function __construct(PaymentProcessor $processor)
    {
        $this->processor = $processor;
    }

    public function checkout(Cart $cart): void
    {
        $result = $this->processor->process($cart->toInvoice());
    }
}
```

---

### 3. Aliased Import

```php
<?php
// src/Services/OrderService.php
// BEFORE

namespace App\Services;

use App\Legacy\Services\PaymentProcessor as Processor;  // ← ALIASED
use App\Models\Order;

class OrderService
{
    public function processOrder(Order $order, Processor $processor): void
    {
        // Alias is used in type hint
        $processor->process($order->getInvoice());
    }

    public function getProcessorStatus(): string
    {
        // Using alias for constant access
        return Processor::STATUS_PENDING;
    }
}
```

```php
<?php
// src/Services/OrderService.php  
// AFTER

namespace App\Services;

use App\Payment\Processing\PaymentProcessor as Processor;  // ← UPDATED, ALIAS PRESERVED
use App\Models\Order;

class OrderService
{
    public function processOrder(Order $order, Processor $processor): void
    {
        $processor->process($order->getInvoice());
    }

    public function getProcessorStatus(): string
    {
        return Processor::STATUS_PENDING;
    }
}
```

---

### 4. Fully Qualified Name (No Use Statement)

```php
<?php
// src/Models/Invoice.php
// BEFORE

namespace App\Models;

class Invoice
{
    private ?\App\Legacy\Services\PaymentProcessor $processor = null;  // ← FQN in type hint

    /**
     * @param \App\Legacy\Services\PaymentProcessor $processor  // ← FQN in PHPDoc
     */
    public function setProcessor(\App\Legacy\Services\PaymentProcessor $processor): void
    {
        $this->processor = $processor;
    }

    public function createProcessor(): \App\Legacy\Services\PaymentProcessor
    {
        return new \App\Legacy\Services\PaymentProcessor('default-key');  // ← FQN in instantiation
    }
}
```

```php
<?php
// src/Models/Invoice.php
// AFTER

namespace App\Models;

class Invoice
{
    private ?\App\Payment\Processing\PaymentProcessor $processor = null;  // ← UPDATED

    /**
     * @param \App\Payment\Processing\PaymentProcessor $processor  // ← UPDATED
     */
    public function setProcessor(\App\Payment\Processing\PaymentProcessor $processor): void
    {
        $this->processor = $processor;
    }

    public function createProcessor(): \App\Payment\Processing\PaymentProcessor
    {
        return new \App\Payment\Processing\PaymentProcessor('default-key');  // ← UPDATED
    }
}
```

---

### 5. PHPDoc Only References

```php
<?php
// src/Events/PaymentCompleted.php
// BEFORE

namespace App\Events;

/**
 * Event fired when payment is completed.
 * 
 * @see \App\Legacy\Services\PaymentProcessor::process()  // ← PHPDoc @see
 */
class PaymentCompleted
{
    /**
     * @var \App\Legacy\Services\PaymentProcessor|null  // ← PHPDoc @var
     */
    public $processor;

    /**
     * @param mixed $result
     * @param \App\Legacy\Services\PaymentProcessor $processor  // ← PHPDoc @param
     * @return void
     */
    public function __construct($result, $processor)
    {
        $this->processor = $processor;
    }
}
```

```php
<?php
// src/Events/PaymentCompleted.php
// AFTER

namespace App\Events;

/**
 * Event fired when payment is completed.
 * 
 * @see \App\Payment\Processing\PaymentProcessor::process()  // ← UPDATED
 */
class PaymentCompleted
{
    /**
     * @var \App\Payment\Processing\PaymentProcessor|null  // ← UPDATED
     */
    public $processor;

    /**
     * @param mixed $result
     * @param \App\Payment\Processing\PaymentProcessor $processor  // ← UPDATED
     * @return void
     */
    public function __construct($result, $processor)
    {
        $this->processor = $processor;
    }
}
```

---

### 6. Test File with Multiple Reference Types

```php
<?php
// src/Tests/PaymentProcessorTest.php
// BEFORE

namespace App\Tests;

use App\Legacy\Services\PaymentProcessor;  // ← Use statement
use PHPUnit\Framework\TestCase;

class PaymentProcessorTest extends TestCase
{
    public function testCanInstantiate(): void
    {
        // Direct instantiation
        $processor = new PaymentProcessor('test-key');
        $this->assertInstanceOf(PaymentProcessor::class, $processor);  // ← ::class constant
    }

    public function testStaticFactory(): void
    {
        // Static method call
        $processor = PaymentProcessor::create('test-key');
        $this->assertNotNull($processor);
    }

    public function testConstants(): void
    {
        // Constant access
        $this->assertEquals('pending', PaymentProcessor::STATUS_PENDING);
        $this->assertEquals('completed', PaymentProcessor::STATUS_COMPLETED);
    }

    public function testInstanceOf(): void
    {
        $processor = new PaymentProcessor('key');
        
        // instanceof check
        if ($processor instanceof PaymentProcessor) {
            $this->assertTrue(true);
        }
    }

    /**
     * @dataProvider processorProvider
     * @param PaymentProcessor $processor  // ← PHPDoc type hint
     */
    public function testWithDataProvider(PaymentProcessor $processor): void
    {
        $this->assertNotNull($processor);
    }

    public function processorProvider(): array
    {
        return [
            'default' => [new PaymentProcessor('key-1')],
            'custom' => [PaymentProcessor::create('key-2')],
        ];
    }
}
```

```php
<?php
// src/Tests/PaymentProcessorTest.php
// AFTER

namespace App\Tests;

use App\Payment\Processing\PaymentProcessor;  // ← UPDATED
use PHPUnit\Framework\TestCase;

// All other references use the short name, so they remain unchanged
// because they resolve through the updated use statement
class PaymentProcessorTest extends TestCase
{
    // ... identical except for the use statement
}
```

---

### 7. Configuration File (String Reference)

```php
<?php
// src/config/services.php
// BEFORE

return [
    'payment' => [
        'processor' => \App\Legacy\Services\PaymentProcessor::class,  // ← ::class in config
        'processor_fqn' => 'App\\Legacy\\Services\\PaymentProcessor',  // ← String FQN
    ],
    
    'bindings' => [
        \App\Legacy\Services\PaymentProcessor::class => function ($container) {
            return new \App\Legacy\Services\PaymentProcessor(
                $container->get('config.payment.api_key')
            );
        },
    ],
];
```

```php
<?php
// src/config/services.php
// AFTER

return [
    'payment' => [
        'processor' => \App\Payment\Processing\PaymentProcessor::class,  // ← UPDATED
        'processor_fqn' => 'App\\Payment\\Processing\\PaymentProcessor',  // ← STRING UPDATED
    ],
    
    'bindings' => [
        \App\Payment\Processing\PaymentProcessor::class => function ($container) {
            return new \App\Payment\Processing\PaymentProcessor(
                $container->get('config.payment.api_key')
            );
        },
    ],
];
```

---

### 8. Legacy Bootstrap (require_once)

```php
<?php
// src/bootstrap.php
// BEFORE

// Legacy autoloading before PSR-4
require_once __DIR__ . '/Legacy/Services/PaymentProcessor.php';  // ← File path

// Later...
$processor = new \App\Legacy\Services\PaymentProcessor($apiKey);
```

```php
<?php
// src/bootstrap.php
// AFTER

require_once __DIR__ . '/Payment/Processing/PaymentProcessor.php';  // ← PATH UPDATED

$processor = new \App\Payment\Processing\PaymentProcessor($apiKey);  // ← FQN UPDATED
```

---

### 9. Same Namespace (Implicit Resolution) - EDGE CASE

```php
<?php
// src/Legacy/Services/PaymentLogger.php
// BEFORE

namespace App\Legacy\Services;

// NO use statement - PaymentProcessor resolves implicitly because same namespace

class PaymentLogger
{
    public function log(PaymentProcessor $processor): void  // ← Implicit namespace resolution
    {
        $status = PaymentProcessor::STATUS_COMPLETED;  // ← Same namespace constant
        echo "Logged processor with status: $status";
    }

    public function createDefault(): PaymentProcessor
    {
        return new PaymentProcessor('default');  // ← Implicit instantiation
    }
}
```

```php
<?php
// src/Legacy/Services/PaymentLogger.php
// AFTER - Must add use statement!

namespace App\Legacy\Services;

use App\Payment\Processing\PaymentProcessor;  // ← NEW USE STATEMENT ADDED

class PaymentLogger
{
    public function log(PaymentProcessor $processor): void
    {
        $status = PaymentProcessor::STATUS_COMPLETED;
        echo "Logged processor with status: $status";
    }

    public function createDefault(): PaymentProcessor
    {
        return new PaymentProcessor('default');
    }
}
```

---

### 10. Group Use Statements

```php
<?php
// src/Services/PaymentFacade.php
// BEFORE

namespace App\Services;

use App\Legacy\Services\{
    PaymentProcessor,      // ← Part of group
    PaymentValidator,
    PaymentResult
};

class PaymentFacade
{
    public function process(): PaymentResult
    {
        $processor = new PaymentProcessor('key');
        return $processor->process(new Invoice());
    }
}
```

```php
<?php
// src/Services/PaymentFacade.php
// AFTER - Group must be split or reorganized

namespace App\Services;

use App\Payment\Processing\PaymentProcessor;  // ← MOVED OUT OF GROUP
use App\Legacy\Services\{
    PaymentValidator,
    PaymentResult
};

class PaymentFacade
{
    public function process(): PaymentResult
    {
        $processor = new PaymentProcessor('key');
        return $processor->process(new Invoice());
    }
}
```

---

## Reference Types Checklist

| Reference Type | Example | Complexity |
|----------------|---------|------------|
| ✅ Use statement | `use App\Old\Class;` | Simple |
| ✅ Aliased use | `use App\Old\Class as Alias;` | Medium |
| ✅ Group use | `use App\Old\{Class, Other};` | Medium |
| ✅ FQN type hint | `\App\Old\Class $param` | Simple |
| ✅ Short type hint | `Class $param` (via use) | Simple |
| ✅ Return type | `function(): Class` | Simple |
| ✅ Property type | `private Class $prop;` | Simple |
| ✅ Nullable type | `?Class $param` | Simple |
| ✅ Union type | `Class\|null $param` | Medium |
| ✅ Intersection | `Class&Interface` | Medium |
| ✅ new instantiation | `new Class()` | Simple |
| ✅ Static call | `Class::method()` | Simple |
| ✅ Constant access | `Class::CONST` | Simple |
| ✅ ::class constant | `Class::class` | Simple |
| ✅ instanceof | `$x instanceof Class` | Simple |
| ✅ catch block | `catch (Class $e)` | Simple |
| ✅ extends | `class X extends Class` | Simple |
| ✅ implements | `class X implements Class` | Simple |
| ✅ trait use | `use ClassTrait;` | Simple |
| ✅ PHPDoc @param | `@param Class $x` | Medium |
| ✅ PHPDoc @return | `@return Class` | Medium |
| ✅ PHPDoc @var | `@var Class` | Medium |
| ✅ PHPDoc @throws | `@throws Class` | Medium |
| ✅ PHPDoc @see | `@see Class::method()` | Medium |
| ✅ PHPDoc generics | `Collection<Class>` | Hard |
| ✅ String FQN | `'App\\Old\\Class'` | Hard |
| ✅ Implicit same-ns | No use, same namespace | Hard |
| ✅ require/include | `require 'path/Class.php'` | Hard |
| ✅ Attributes | `#[Class(param)]` | Medium |

---

## Implementation Notes

### Algorithm for Class Move

```rust
pub struct MoveClassRefactoring {
    pub source_file: PathBuf,
    pub target_directory: PathBuf,
    pub new_namespace: String,
}

impl MoveClassRefactoring {
    pub fn execute(&self, project: &Project) -> Vec<FileEdit> {
        let mut edits = Vec::new();
        
        // 1. Parse source file, extract class info
        let class_info = self.extract_class_info(&self.source_file);
        let old_fqn = class_info.fully_qualified_name();
        let new_fqn = format!("{}\\{}", self.new_namespace, class_info.name);
        
        // 2. Find ALL references across the project
        let references = project.find_references(&old_fqn);
        
        // 3. Group references by file
        let refs_by_file = references.group_by(|r| r.file.clone());
        
        // 4. Generate edits for each file
        for (file, refs) in refs_by_file {
            let file_edits = self.generate_edits_for_file(&file, &refs, &old_fqn, &new_fqn);
            edits.extend(file_edits);
        }
        
        // 5. Update the class file itself (namespace declaration)
        edits.push(self.update_class_file(&class_info, &new_fqn));
        
        // 6. Move the file
        edits.push(FileEdit::Move {
            from: self.source_file.clone(),
            to: self.target_directory.join(format!("{}.php", class_info.name)),
        });
        
        edits
    }
    
    fn generate_edits_for_file(
        &self,
        file: &Path,
        refs: &[Reference],
        old_fqn: &str,
        new_fqn: &str,
    ) -> Vec<FileEdit> {
        let mut edits = Vec::new();
        let source = std::fs::read_to_string(file).unwrap();
        let ast = parse(&source);
        
        // Check if file is in same namespace (implicit resolution case)
        let file_namespace = ast.get_namespace();
        let was_same_namespace = file_namespace == self.old_namespace();
        let is_same_namespace = file_namespace == self.new_namespace;
        
        // Determine if we need to add/update/remove use statement
        let has_use_statement = ast.has_use_for(&old_fqn);
        let uses_implicit = refs.iter().any(|r| r.is_implicit_namespace_resolution());
        
        if has_use_statement {
            // Update existing use statement
            edits.push(self.update_use_statement(&ast, old_fqn, new_fqn));
        } else if uses_implicit && !is_same_namespace {
            // Must ADD a use statement - class moved out of same namespace
            edits.push(self.add_use_statement(&ast, new_fqn));
        }
        
        // Update all FQN references
        for ref_ in refs.iter().filter(|r| r.is_fully_qualified()) {
            edits.push(Edit::replace(ref_.span, new_fqn));
        }
        
        // Update string references
        for ref_ in refs.iter().filter(|r| r.is_string_reference()) {
            let escaped_new = new_fqn.replace("\\", "\\\\");
            edits.push(Edit::replace(ref_.span, &escaped_new));
        }
        
        // Update PHPDoc references
        for ref_ in refs.iter().filter(|r| r.is_phpdoc()) {
            edits.push(self.update_phpdoc_reference(ref_, old_fqn, new_fqn));
        }
        
        // Update require/include paths
        for ref_ in refs.iter().filter(|r| r.is_file_include()) {
            edits.push(self.update_include_path(ref_, &self.target_directory));
        }
        
        edits
    }
}
```

### Reference Detection Strategy

```rust
pub enum ReferenceKind {
    UseStatement { aliased: bool, alias: Option<String>, grouped: bool },
    TypeHint { nullable: bool, union: bool },
    ReturnType,
    PropertyType,
    Instantiation,       // new Class()
    StaticAccess,        // Class::method() or Class::$prop
    ConstantAccess,      // Class::CONST
    ClassConstant,       // Class::class
    InstanceOf,
    Catch,
    Extends,
    Implements,
    TraitUse,
    PhpDoc { tag: String },  // @param, @return, @var, etc.
    StringLiteral,       // 'App\\Class' in configs
    Attribute,           // #[Class]
    ImplicitNamespace,   // Same namespace, no use statement
    FileInclude,         // require/include path
}

pub struct Reference {
    pub file: PathBuf,
    pub span: Span,
    pub kind: ReferenceKind,
    pub text: String,
}
```

---

## Test Execution

```bash
# Run the class move refactoring
php-refactor move-class \
    --from src/Legacy/Services/PaymentProcessor.php \
    --to src/Payment/Processing/ \
    --namespace "App\Payment\Processing" \
    --dry-run

# Expected output:
# 
# Moving class: App\Legacy\Services\PaymentProcessor
#           to: App\Payment\Processing\PaymentProcessor
#
# Files to modify: 10
#
# src/Legacy/Services/PaymentProcessor.php
#   - Update namespace declaration
#   - Move file to src/Payment/Processing/PaymentProcessor.php
#
# src/Controllers/CheckoutController.php
#   - Update use statement (line 5)
#
# src/Services/OrderService.php  
#   - Update aliased use statement (line 5)
#
# src/Models/Invoice.php
#   - Update FQN type hint (line 7)
#   - Update FQN in PHPDoc (line 10)
#   - Update FQN type hint (line 12)
#   - Update FQN return type (line 17)
#   - Update FQN instantiation (line 19)
#
# src/Events/PaymentCompleted.php
#   - Update PHPDoc @see (line 6)
#   - Update PHPDoc @var (line 11)
#   - Update PHPDoc @param (line 17)
#
# src/Tests/PaymentProcessorTest.php
#   - Update use statement (line 5)
#
# src/config/services.php
#   - Update ::class reference (line 5)
#   - Update string FQN (line 6)
#   - Update ::class reference (line 10)
#   - Update FQN instantiation (line 11)
#
# src/bootstrap.php
#   - Update require_once path (line 4)
#   - Update FQN instantiation (line 7)
#
# src/Legacy/Services/PaymentLogger.php
#   - ADD use statement for App\Payment\Processing\PaymentProcessor
#
# src/Services/PaymentFacade.php
#   - Split group use statement
#   - Add separate use statement
#
# Total edits: 23
```

---

## Success Criteria

1. **All reference types** from the checklist are correctly updated
2. **No broken code** — project should pass static analysis after refactoring
3. **Format preservation** — only changed lines should differ
4. **Implicit namespace** case correctly adds use statement
5. **Group use statements** are properly split when needed
6. **String FQNs** in configs are detected and updated
7. **PHPDoc** references are updated (including generics like `Collection<Class>`)
8. **File path** in require/include is updated
9. **Dry run** shows accurate preview of all changes
