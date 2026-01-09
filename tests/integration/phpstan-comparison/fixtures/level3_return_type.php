<?php
// Level 3 test - Return type and property type validation

class Level3Test {
    public string $name;
    public int $count;

    public function getCount(): int {
        return $this->count;
    }

    // Missing return statement
    public function getName(): string {
        // No return - should error
    }

    // Returns wrong type
    public function getWrongType(): string {
        return 123; // Error: returns int, expects string
    }

    // Nullable return type with non-nullable return
    public function maybeNull(): ?string {
        return "hello";
    }

    // Void function with return value
    public function noReturn(): void {
        return "oops"; // Error: void function returns value
    }
}

// Function with missing return
function missingReturn(): int {
    // No return statement
}

// Function with wrong return type
function wrongReturn(): string {
    return 42;
}

// Correct function
function correctReturn(): int {
    return 42;
}
