<?php
// Both rules should apply here
array_push($arr, $value);
if (is_null($x)) {
    array_push($results, 'null value');
}

// Negated is_null
if (!is_null($user)) {
    array_push($users, $user);
}

// Mixed usage
$isValid = !is_null($data) && count($data) > 0;
