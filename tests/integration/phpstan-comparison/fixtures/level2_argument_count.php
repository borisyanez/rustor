<?php
// Test: Wrong argument counts (Level 2)
// Expected errors:
// - Line 14: Function requiresTwo() invoked with 1 parameter, 2 required
// - Line 17: Function requiresTwo() invoked with 3 parameters, 2 required
// - Line 23: Method TestClass::methodWithArgs() invoked with 0 parameters, 2 required

function requiresTwo(string $a, int $b): void {
}

// OK - correct argument count
requiresTwo('test', 42);

// Too few arguments
requiresTwo('test');

// Too many arguments
requiresTwo('test', 42, 'extra');

class TestClass {
    public function methodWithArgs(string $a, int $b): void {}
}

$obj = new TestClass();
$obj->methodWithArgs(); // Too few
$obj->methodWithArgs('a', 1); // OK
