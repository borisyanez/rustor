<?php
// Level 1 test - Undefined variables

function testUndefinedVar() {
    echo $undefinedVar;
}

function testMaybeUndefined($flag) {
    if ($flag) {
        $x = 1;
    }
    echo $x; // $x might be undefined
}

class MagicMethodTest {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }

    public function test() {
        echo $this->magicProp; // Should be OK with __get
    }
}
