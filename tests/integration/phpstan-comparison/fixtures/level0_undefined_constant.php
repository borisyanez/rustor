<?php
// Test: Undefined class constant access (Level 0)
// Expected errors:
// - Line 14: Access to undefined constant TestClass::UNDEFINED_CONST

class TestClass {
    public const DEFINED_CONST = 'value';
}

// OK - defined constant
echo TestClass::DEFINED_CONST;

// Undefined constant
echo TestClass::UNDEFINED_CONST;

// OK - special ::class constant
echo TestClass::class;
