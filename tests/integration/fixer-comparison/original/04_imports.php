<?php

namespace App\Test;

use Zebra\Something;
use Alpha\Something;
use \Leading\Slash;
use Beta\Something;
use function strlen;
use const PHP_VERSION;
use Gamma\{First, Second, Third};

class ImportsTest
{
    public function test()
    {
        return new Something();
    }
}
