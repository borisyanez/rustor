<?php
// Test: Undefined property access (Level 0)
// Expected errors:
// - Line 16: Access to an undefined property TestClass::$undefinedProp

class TestClass {
    public string $definedProp = 'test';

    public function test(): void {
        echo $this->definedProp; // OK
    }
}

$obj = new TestClass();

// Undefined property
echo $obj->undefinedProp;

// OK - defined property
echo $obj->definedProp;
