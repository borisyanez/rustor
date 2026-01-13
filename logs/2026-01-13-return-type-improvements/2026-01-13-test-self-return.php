<?php

class Foo {
    public static function factory(): self {
        return new Foo();  // Should NOT be flagged
    }

    public static function create(): static {
        return new Foo();  // Should NOT be flagged
    }

    public function getInstance(): self {
        return new Foo();  // Should NOT be flagged
    }

    public function wrong(): self {
        return new Bar();  // SHOULD be flagged (different class)
    }
}

class Bar {}
