<?php
// These should be transformed (return value not used)
array_push($arr, 'value');
array_push($items, $x);

// These should NOT be transformed (return value used)
$count = array_push($arr, 'value');
$len = array_push($items, $x);
if (array_push($arr, $val) > 5) { }
echo array_push($arr, 'test');
return array_push($results, $item);
doSomething(array_push($arr, $val));
$counts[] = array_push($arr, $val);
