<?php
// Simple test cases for array_push transformation

$items = [];

// Should be transformed
array_push($items, 'foo');
array_push($items, $bar);
array_push($data['key'], getValue());

// Should NOT be transformed (multiple values)
array_push($items, 'a', 'b', 'c');

// Should NOT be transformed (return value used)
$count = array_push($items, 'foo');
