<?php
// Level 5 test - Argument type validation

class Level5Test {
    public function expectsString(string $s): void {
        echo $s;
    }

    public function expectsInt(int $n): void {
        echo $n;
    }

    public function expectsArray(array $arr): void {
        print_r($arr);
    }

    public function testWrongTypes(): void {
        $this->expectsString(123);        // Error: expects string, int given
        $this->expectsInt("hello");       // Error: expects int, string given
        $this->expectsArray("not array"); // Error: expects array, string given
    }

    public function testNullToNonNullable(): void {
        $this->expectsString(null);       // Error: expects string, null given
    }

    public function testWithVariable(): void {
        $number = 42;
        $this->expectsString($number);    // Error: expects string, int given
    }
}

function takesCallback(callable $cb): void {
    $cb();
}

function testCallableError(): void {
    takesCallback("not_a_function");      // Error: expects callable
}

function takesObject(object $obj): void {
    var_dump($obj);
}

function testObjectError(): void {
    takesObject("string");                // Error: expects object, string given
}
