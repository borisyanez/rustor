<?php

namespace App\Test;

class CasingTest
{
    PUBLIC $publicProp;
    PRIVATE $privateProp;
    PROTECTED $protectedProp;
    STATIC $staticProp;
    CONST MY_CONST = 1;

    PUBLIC FUNCTION test()
    {
        $a = TRUE;
        $b = FALSE;
        $c = NULL;

        IF ($a) {
            RETURN SELF::MY_CONST;
        } ELSEIF ($b) {
            RETURN STATIC::$staticProp;
        } ELSE {
            RETURN PARENT::test();
        }

        FOREACH ($arr AS $item) {
            ECHO $item;
        }

        WHILE (TRUE) {
            BREAK;
        }

        SWITCH ($c) {
            CASE 1:
                BREAK;
            DEFAULT:
                BREAK;
        }

        TRY {
            THROW NEW Exception();
        } CATCH (Exception $e) {
            ECHO $e;
        } FINALLY {
            ECHO 'done';
        }
    }
}
