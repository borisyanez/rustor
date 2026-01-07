<?php
// Type conversion functions to be replaced with casts
$str = strval($num);
$int = intval($input);
$float = floatval($price);
$bool = boolval($flag);

// Should skip: intval with base
$hex = intval($hexStr, 16);

// Multiple in one line
$result = intval($a) + floatval($b);
