<?php
// Test: Constructor argument count validation (Level 0)
// Expected errors:
// - Line 15: Class TestClass constructor invoked with 0 parameters, 2 required.
// - Line 18: Class TestClass constructor invoked with 3 parameters, 2 required.

class TestClass {
    public function __construct(string $a, int $b) {
    }
}

// OK - correct argument count
$obj = new TestClass('test', 42);

// Too few arguments
$obj2 = new TestClass();

// Too many arguments
$obj3 = new TestClass('test', 42, 'extra');

// OK - class with no constructor (implicitly zero args)
class NoConstructor {
    public function method(): void {}
}

$obj4 = new NoConstructor();
