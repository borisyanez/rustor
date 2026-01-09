<?php

namespace App\Test;

class VisibilityTest
{
    var $oldStyle;
    $noVisibility;

    public $public;
    protected $protected;
    private $private;

    static $staticNoVisibility;
    public static $publicStatic;
    static public $staticPublic;

    function noVisibilityMethod()
    {
        return 1;
    }

    public function publicMethod()
    {
        return 2;
    }

    protected function protectedMethod()
    {
        return 3;
    }

    private function privateMethod()
    {
        return 4;
    }

    static function staticNoVisibility()
    {
        return 5;
    }

    public static function publicStatic()
    {
        return 6;
    }

    static public function staticPublic()
    {
        return 7;
    }

    final public function finalPublic()
    {
        return 8;
    }

    public final function publicFinal()
    {
        return 9;
    }

    abstract public function abstractPublic();
}
