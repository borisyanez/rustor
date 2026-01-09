<?php

namespace App\Test;

class FunctionsTest
{
    public function noArgs()
    {
        return 1;
    }

    public function withArgs($a,$b,$c)
    {
        return $a + $b + $c;
    }

    public function withTypes(int $a,string $b,?array $c): int
    {
        return $a;
    }

    public function withDefaults($a = 1,$b = 'test',$c = [])
    {
        return $a;
    }

    public function multiline(
        $a,
        $b,
        $c
    ) {
        return $a;
    }

    public function returnTypes(): string
    {
        return 'test';
    }

    public function nullableReturn(): ?string
    {
        return null;
    }

    public function unionTypes(int|string $a): int|string
    {
        return $a;
    }
}

function globalFunction ($a , $b) {
    return $a + $b;
}
