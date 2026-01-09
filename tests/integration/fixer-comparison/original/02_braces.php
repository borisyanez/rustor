<?php

namespace App\Test;

class BracesTest {
    public function sameLine() {
        if($a){
            return 1;
        }
        elseif($b){
            return 2;
        }
        else{
            return 3;
        }
    }

    public function nextLine()
    {
        while($x)
        {
            $x--;
        }

        for($i=0;$i<10;$i++)
        {
            echo $i;
        }

        foreach($arr as $k=>$v)
        {
            echo $v;
        }

        switch($type)
        {
            case 'a':
                break;
            default:
                break;
        }

        try
        {
            throw new Exception();
        }
        catch(Exception $e)
        {
            echo $e->getMessage();
        }
        finally
        {
            echo "done";
        }
    }
}
