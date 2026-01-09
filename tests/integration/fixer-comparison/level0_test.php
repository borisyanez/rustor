<?php
// Level 0 test - Basic syntax and undefined symbols

// Undefined function call
unknownFunction();

// Undefined class instantiation
$obj = new UndefinedClass();

// Undefined constant
echo UNDEFINED_CONST;

// Function with too many args (rustor-specific)
strlen("test", "extra");

// Undefined static method call
UnknownClass::staticMethod();

// Undefined class constant
echo SomeClass::UNDEFINED_CONST;
