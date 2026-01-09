<?php
// Test: Undefined method calls (Level 0)
// Expected errors:
// - Line 14: Call to an undefined method TestClass::undefinedMethod()

class TestClass {
    public function definedMethod(): void {
    }
}

$obj = new TestClass();

// Undefined method
$obj->undefinedMethod();

// OK - defined method
$obj->definedMethod();
