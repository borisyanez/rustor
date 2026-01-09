<?php

namespace App\Test;

class StringsTest
{
    public function quotes()
    {
        $a = "simple string";
        $b = 'simple string';
        $c = "string with $var";
        $d = "string with {$var}";
        $e = "string with {$obj->prop}";
        $f = 'string with literal $var';
    }

    public function heredoc()
    {
        $a = <<<EOT
heredoc string
EOT;

        $b = <<<'EOT'
nowdoc string
EOT;
    }

    public function multiline()
    {
        $a = "first line\n" .
             "second line\n" .
             "third line";

        $b = 'first line'
           . 'second line'
           . 'third line';
    }
}
