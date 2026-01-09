<?php
// Level 4 test - Dead code detection

class Level4Test {
    public function unreachableCode(): int {
        return 42;
        echo "This is unreachable"; // Dead code after return
    }

    public function alwaysFalseInstanceof(string $value): void {
        if ($value instanceof stdClass) { // Always false - string can't be stdClass
            echo "impossible";
        }
    }

    public function deadElseBranch(int $x): string {
        if ($x > 0) {
            return "positive";
        } elseif ($x <= 0) {
            return "non-positive";
        } else {
            return "dead branch"; // Unreachable - covered by conditions above
        }
    }

    public function alwaysTrueCondition(string $s): void {
        if (is_string($s)) { // Always true - $s is typed as string
            echo "always executes";
        }
    }

    public function unusedVariable(): void {
        $unused = 42; // Unused variable
        echo "done";
    }

    public function deadCodeInSwitch(int $x): string {
        switch ($x) {
            case 1:
                return "one";
                break; // Dead code after return
            default:
                return "other";
        }
    }
}

function unreachableAfterThrow(): void {
    throw new Exception("error");
    echo "never reached"; // Dead code after throw
}
