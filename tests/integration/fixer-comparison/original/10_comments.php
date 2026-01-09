<?php

namespace App\Test;

/**
 * Class documentation
 *@param string $test
 *@return void
 */
class CommentsTest
{
    //single line comment without space
    // single line comment with space
    //  single line comment with extra space

    # hash comment without space
    # hash comment with space

    /* inline block */
    /*inline block without space*/

    /**
     *PHPDoc without space after asterisk
     */
    public function test()
    {
        $a = 1; // trailing comment
        $b = 2; //trailing comment without space
        $c = 3;// comment directly after code
    }
}
