<?php
namespace App\Test;
use Zebra\A;use Alpha\B;
use Beta\C;
class MixedTest {
    const A=1;const B=2;
    public $a,$b;
    public function test($x,$y,$z){
        if($x&&$y||$z){
            return $x?$y:$z;
        }elseif($x==$y){
            foreach($arr as $k=>$v){
                echo $v;
            }
        }else{
            switch($x){
                case 1:break;
                default:break;
            }
        }
        $arr=['a'=>1,'b'=>2,'c'=>3];
        $fn=fn($n)=>$n*2;
        return $arr??[];
    }
}
