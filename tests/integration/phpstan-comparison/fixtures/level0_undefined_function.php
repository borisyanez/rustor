<?php
// Test: Undefined function calls (Level 0)
// Expected errors:
// - Line 8: Function undefined_function not found

function defined_function(): void {
}

undefined_function();

defined_function(); // OK
strlen("test"); // OK - builtin
