<?php

namespace App\Test;

class ArraysTest
{
    public function oldSyntax()
    {
        $a = array();
        $b = array(1, 2, 3);
        $c = array('a' => 1, 'b' => 2);
        $d = array(
            'first',
            'second',
            'third',
        );
    }

    public function newSyntax()
    {
        $a = [];
        $b = [1,2,3];
        $c = [ 1 , 2 , 3 ];
        $d = ['a'=>1,'b'=>2];
        $e = [ 'a' => 1 , 'b' => 2 ];
    }

    public function access()
    {
        $a = $arr[0];
        $b = $arr['key'];
    }

    public function trailing()
    {
        $a = [1, 2, 3];
        $b = [1, 2, 3,];
        $c = [
            1,
            2,
            3
        ];
        $d = [
            1,
            2,
            3,
        ];
    }
}
