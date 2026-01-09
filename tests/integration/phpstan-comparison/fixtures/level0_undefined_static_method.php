<?php
// Test: Undefined static method calls (Level 0)
// Expected errors:
// - Line 14: Call to an undefined static method TestClass::undefinedStatic()

class TestClass {
    public static function definedStatic(): void {
    }
}

// OK - defined static method
TestClass::definedStatic();

// Undefined static method
TestClass::undefinedStatic();
