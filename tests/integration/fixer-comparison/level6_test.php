<?php
// Level 6 test - Missing typehints

class Level6Test {
    // Missing property type
    public $untyped;

    // Missing parameter type
    public function missingParamType($param): void {
        echo $param;
    }

    // Missing return type
    public function missingReturnType(string $s) {
        return $s . "!";
    }

    // Missing both parameter and return type
    public function missingBoth($a, $b) {
        return $a + $b;
    }

    // Properly typed (no error)
    public function fullyTyped(string $s): string {
        return $s;
    }
}

// Function missing parameter type
function noParamType($x): int {
    return $x * 2;
}

// Function missing return type
function noReturnType(int $x) {
    return $x * 2;
}

// Function with no types at all
function noTypes($a, $b) {
    return $a + $b;
}

// Properly typed function (no error)
function fullyTypedFunc(int $a, int $b): int {
    return $a + $b;
}
