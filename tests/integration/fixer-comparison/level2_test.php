<?php
// Level 2 test - Type-aware method and property checks

class User {
    public string $name;
    public int $age;

    public function getName(): string {
        return $this->name;
    }
}

function testTypedParam(User $user) {
    // Valid
    echo $user->name;
    echo $user->getName();

    // Invalid - undefined property on typed param
    echo $user->undefinedProp;

    // Invalid - undefined method on typed param
    $user->undefinedMethod();
}

function testWithObject(object $obj) {
    // Cannot validate on generic 'object'
    echo $obj->anyProp;
}
