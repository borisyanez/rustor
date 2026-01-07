<?php
// These should be transformed (2 args)
array_push($arr, 'single');
array_push($items, $value);

// These should NOT be transformed (3+ args)
array_push($arr, 'one', 'two');
array_push($arr, 1, 2, 3);
array_push($items, $a, $b, $c, $d);

// Mixed in same file
array_push($result, getValue());  // should transform
array_push($result, 'x', 'y');    // should NOT transform
