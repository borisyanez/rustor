<?php
// Test: Undefined variable usage (Level 1)
// Expected errors:
// - Line 8: Variable $undefined might not be defined
// - Line 14: Variable $conditionalVar might not be defined

// Simple undefined variable
echo $undefined;

// Conditional variable definition
if (rand(0, 1)) {
    $conditionalVar = 'test';
}
echo $conditionalVar;

// OK - defined variable
$defined = 'value';
echo $defined;

// OK - superglobals
echo $_GET['test'] ?? '';
echo $_SERVER['REQUEST_URI'] ?? '';
