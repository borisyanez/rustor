<?php

namespace App\Test;

class ControlStructuresTest
{
    public function ifStatements()
    {
        if($a){}
        if( $a ){}
        if ($a) {}

        if($a):
            echo 1;
        endif;

        if ($a) echo 1;

        if ($a)
            echo 1;

        if ($a) { echo 1; }
    }

    public function loops()
    {
        while($x){}
        while( $x ){}

        for($i=0;$i<10;$i++){}
        for( $i = 0; $i < 10; $i++ ){}
        for ($i = 0; $i < 10; $i++) {}

        foreach($arr as $v){}
        foreach( $arr as $v ){}
        foreach($arr as $k=>$v){}
        foreach ($arr as $k => $v) {}

        do {} while($x);
        do {} while( $x );
        do {} while ($x);
    }

    public function switchCase()
    {
        switch($a){}
        switch( $a ){}
        switch ($a) {}

        switch ($a) {
            case 1;
                break;
            case 2:
                break;
            default;
                break;
        }
    }

    public function tryCatch()
    {
        try{}catch(Exception $e){}
        try {} catch (Exception $e) {}
        try {} catch (Exception|Error $e) {}

        try {
        } catch (Exception $e) {
        } finally {
        }
    }
}
