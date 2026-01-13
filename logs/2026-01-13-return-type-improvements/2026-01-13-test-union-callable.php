<?php

// Test 1: Union type with null return
function testNullReturn(): int|null {
    return null;  // Should NOT error
}

// Test 2: Union type with string return
function testStringReturn(): string|int {
    return "hello";  // Should NOT error
}

// Test 3: Union type with int return
function testIntReturn(): string|int {
    return 42;  // Should NOT error
}

// Test 4: Multiple union members
function testMultiUnion(): int|string|null {
    return null;  // Should NOT error
}

function testMultiUnion2(): int|string|null {
    return 5;  // Should NOT error
}

function testMultiUnion3(): int|string|null {
    return "text";  // Should NOT error
}

// Test 5: Callable/Closure compatibility
function testCallable(): callable {
    return function() { return 42; };  // Should NOT error - Closure is callable
}

function compose(callable $f, callable $g): callable {
    return function($x) use ($f, $g) {  // Should NOT error
        return $f($g($x));
    };
}

// Test 6: Union with array
function testArrayOrNull(): array|null {
    return null;  // Should NOT error
}

function testArrayOrNull2(): array|null {
    return [];  // Should NOT error
}

// Test 7: These SHOULD still error (wrong type)
function testWrongType(): int {
    return "wrong";  // SHOULD error - string is not int
}

function testWrongUnionMember(): int|string {
    return 3.14;  // SHOULD error - float is not in the union
}
