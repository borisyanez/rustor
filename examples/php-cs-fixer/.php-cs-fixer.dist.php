<?php

// https://mlocati.github.io/php-cs-fixer-configurator/

$finder = PhpCsFixer\Finder::create()
    ->in(getcwd())
    ->ignoreVCSIgnored(true)
    ->exclude([
        'bootstrap',
        'config',
        'db/migrations',
        'storage',
        'public',
        'risk',
        'scripts',
        'tools/adminer',
        'www/tcpdf',
    ])
    ->notName('__CG__*.php')
    ->notName('*.blade.php')
    ->notPath('server.php');

return (new PhpCsFixer\Config())
    ->setParallelConfig(PhpCsFixer\Runner\Parallel\ParallelConfigFactory::detect(null, 600))
    ->setLineEnding("\n")
    ->setFinder($finder)
    ->setRules([
        '@PSR12' => true,
        'align_multiline_comment' => true,
        'array_indentation' => true,
        'array_syntax' => ['syntax' => 'short'],
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/php_tag/blank_line_after_opening_tag.rst
        'blank_line_after_opening_tag' => true,
        'blank_line_before_statement' => [
            'statements' => [
                'return',
                'throw',
            ],
        ],
        'braces_position' => [
            'allow_single_line_anonymous_functions' => true,
            'allow_single_line_empty_anonymous_classes' => true,
            'anonymous_classes_opening_brace' => 'same_line',
            'anonymous_functions_opening_brace' => 'same_line',
            'classes_opening_brace' => 'same_line',
            'control_structures_opening_brace' => 'same_line',
            'functions_opening_brace' => 'same_line',
        ],
        'single_line_empty_body' => true,
        'combine_consecutive_unsets' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/class_notation/class_attributes_separation.rst
        'class_attributes_separation' => [
            'elements' => [
                'method' => 'one',
            ],
        ],
        'concat_space' => ['spacing' => 'one'],
        'declare_equal_normalize' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/function_notation/function_typehint_space.rst
        'type_declaration_spaces' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/control_structure/include.rst
        'include' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/php_tag/linebreak_after_opening_tag.rst
        'linebreak_after_opening_tag' => true,
        'lowercase_cast' => true,
        'method_argument_space' => ['on_multiline' => 'ignore'],
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/whitespace/method_chaining_indentation.rst
        'method_chaining_indentation' => true,
        'multiline_whitespace_before_semicolons' => false,
        'no_blank_lines_after_class_opening' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/php_tag/no_closing_tag.rst
        'no_closing_tag' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/whitespace/no_extra_blank_lines.rst
        'no_extra_blank_lines' => [
            'tokens' => [
                'extra',
                'curly_brace_block',
                'parenthesis_brace_block',
                'square_brace_block',
                'throw',
                'use',
                'return',
            ],
        ],
        'no_multiline_whitespace_around_double_arrow' => true,
        'no_spaces_around_offset' => true,
        'no_trailing_comma_in_singleline' => true,
        'no_unused_imports' => true,
        'no_whitespace_before_comma_in_array' => true,
        'no_whitespace_in_blank_line' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/operator/not_operator_with_successor_space.rst
        'not_operator_with_successor_space' => false,
        'object_operator_without_whitespace' => true,
        'ordered_imports' => [
            // 'sort_algorithm' => 'length',
            'sort_algorithm' => 'alpha',
        ],
        'phpdoc_single_line_var_spacing' => true,
        'phpdoc_summary' => true,
        'phpdoc_trim' => true,
        'phpdoc_types' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/php_unit/php_unit_method_casing.rst
        'php_unit_method_casing' => ['case' => 'camel_case'],
        'return_type_declaration' => [
            'space_before' => 'none',
        ],
        // This is a good rule, but we should wait to start using it until we are comfortable with CS Fixer
        //  'self_accessor' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/return_notation/simplified_null_return.rst
        'simplified_null_return' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/class_notation/single_class_element_per_statement.rst
        'single_class_element_per_statement' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/string_notation/single_quote.rst
        'single_quote' => true,
        'single_line_comment_style' => [
            'comment_types' => ['hash'],
        ],
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/operator/standardize_not_equals.rst
        'standardize_not_equals' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/operator/ternary_operator_spaces.rst
        'ternary_operator_spaces' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/operator/ternary_to_null_coalescing.rst
        'ternary_to_null_coalescing' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/control_structure/trailing_comma_in_multiline.rst
        'trailing_comma_in_multiline' => [
            'elements' => [
                'arrays',
            ],
        ],
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/array_notation/trim_array_spaces.rst
        'trim_array_spaces' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/control_structure/trailing_comma_in_multiline.rst
        'unary_operator_spaces' => true,
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/class_notation/visibility_required.rst
        'visibility_required' => [
            'elements' => [
                'property',
                'method',
                'const',
            ],
        ],
        // https://github.com/FriendsOfPHP/PHP-CS-Fixer/blob/master/doc/rules/array_notation/whitespace_after_comma_in_array.rst
        'whitespace_after_comma_in_array' => true,
    ]);

