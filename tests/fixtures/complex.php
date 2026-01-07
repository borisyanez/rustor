<?php
namespace App\Services;

class DataProcessor
{
    private array $results = [];
    private array $errors = [];

    public function process(array $data): void
    {
        foreach ($data as $item) {
            // Should be transformed
            array_push($this->results, $item);
        }

        // Nested in conditionals
        if ($this->shouldProcess()) {
            array_push($this->results, 'processed');
        } else {
            array_push($this->errors, 'skipped');
        }

        // In try-catch
        try {
            array_push($this->results, $this->transform($data));
        } catch (Exception $e) {
            array_push($this->errors, $e->getMessage());
        }
    }

    public function addItem(string $item): void
    {
        // Simple case in method
        array_push($this->results, $item);
    }

    private function shouldProcess(): bool
    {
        return true;
    }

    private function transform(array $data): string
    {
        return json_encode($data);
    }
}

// Function outside class
function processData(array &$arr, $value): void
{
    array_push($arr, $value);
}

// In while loop
$i = 0;
while ($i < 10) {
    array_push($items, $i);
    $i++;
}

// In for loop
for ($j = 0; $j < 5; $j++) {
    array_push($items, $j * 2);
}

// In switch
switch ($type) {
    case 'add':
        array_push($items, $newItem);
        break;
    case 'remove':
        array_pop($items);
        break;
}
