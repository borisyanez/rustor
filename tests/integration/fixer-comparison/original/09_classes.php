<?php

namespace App\Test;

abstract class AbstractClass {
}

final class FinalClass {
}

class ParentClass {
}

class ChildClass extends ParentClass implements InterfaceA,InterfaceB {
    use TraitA,TraitB;
    use TraitC, TraitD;

    const CONST_A=1;
    const CONST_B = 2;

    public $a,$b,$c;
    protected $d = 1, $e = 2;
    private $f;

    public static $staticProp;
    private readonly string $readonlyProp;

    public function __construct(
        public string $promoted,
        private int $privatePromoted
    ) {
        $this->f = 1;
    }
}

interface InterfaceA {
    public function methodA();
}

interface InterfaceB {
}

trait TraitA {
}

trait TraitB {
}

trait TraitC {
}

trait TraitD {
}
