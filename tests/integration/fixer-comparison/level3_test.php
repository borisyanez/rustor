<?php
// Level 3 test - Return type issues

class Level3Test {
    // Missing return type hint but function always returns int
    public function missingReturnType() {
        return 42;
    }

    // Missing return type hint but function always returns void
    public function noReturnType($x) {
        if ($x > 0) {
            echo $x;
        }
    }

    // Returns int but declared void - ERROR
    public function invalidVoidReturn(): void {
        return 42; // Error: function returns value but declared void
    }

    // Void function with no side effects (purity check)
    public function uselessVoid(): void {
        $x = 1 + 2; // No side effects - warning
    }

    // Missing return in non-void function
    public function missingReturn(): int {
        // Error: function should return int but doesn't always return
    }

    // Returns wrong type - should return int but returns string
    public function wrongReturnType(): int {
        return "string"; // Error: wrong return type
    }

    // Inconsistent return types
    public function inconsistentReturns($flag) {
        if ($flag) {
            return 42;
        }
        return "string"; // Inconsistent
    }
}

// Function-level checks
function voidWithReturn(): void {
    return true; // Error
}

// Useless void function with no side effects
function uselessFunction(): void {
    $a = 1;
    $b = $a + 1; // No side effects
}
