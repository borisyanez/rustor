<?php

namespace App\Test;

class OperatorsTest
{
    public function assignment()
    {
        $a=1;
        $b= 2;
        $c =3;
        $d = 4;

        $a+=1;
        $b-=1;
        $c*=2;
        $d/=2;
        $e.='string';
        $f??='default';
    }

    public function comparison()
    {
        if ($a==$b) { }
        if ($a===$b) { }
        if ($a!=$b) { }
        if ($a!==$b) { }
        if ($a<$b) { }
        if ($a>$b) { }
        if ($a<=$b) { }
        if ($a>=$b) { }
        if ($a<>$b) { }
        if ($a<=>$b) { }
    }

    public function logical()
    {
        if ($a&&$b) { }
        if ($a||$b) { }
        if ($a and $b) { }
        if ($a or $b) { }
        if ($a xor $b) { }
        if (!$a) { }
    }

    public function ternary()
    {
        $a=$b?$c:$d;
        $a = $b ? $c : $d;
        $a=$b?:$d;
        $a??$b;
    }

    public function concatenation()
    {
        $a='hello'.'world';
        $a = 'hello' . 'world';
        $a='hello' .'world';
        $a='hello'. 'world';
    }

    public function arrow()
    {
        $arr = ['a'=>1, 'b'=>2];
        foreach ($arr as $k=>$v) { }
        $fn = fn($x)=>$x * 2;
        $fn = fn($x) => $x * 2;
    }
}
