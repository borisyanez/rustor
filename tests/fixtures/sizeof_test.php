<?php
// sizeof should be replaced with count
$len = sizeof($arr);
if (sizeof($items) > 0) {
    echo "Has items";
}
for ($i = 0; $i < sizeof($data); $i++) {
    process($data[$i]);
}
