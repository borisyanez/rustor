<?php
// Test: Undefined class references (Level 0)
// Expected errors:
// - Line 12: Instantiated class UndefinedClass not found
// - Line 15: Class UndefinedParent not found

class DefinedClass {
    public function test(): void {}
}

// Undefined class instantiation
$obj = new UndefinedClass();

// Undefined parent class
class Child extends UndefinedParent {}

// OK - defined class
$defined = new DefinedClass();

// OK - builtin class
$date = new DateTime();
