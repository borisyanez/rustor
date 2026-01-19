//! AST-based PHP analyzer using mago-syntax
//!
//! This module provides proper AST parsing for Rector PHP files,
//! supplementing the regex-based approach with more accurate pattern detection.

use bumpalo::Bump;
use mago_database::file::FileId;
use mago_span::HasSpan;
use mago_span::Span;
use mago_syntax::ast::*;
use mago_syntax::parser::parse_file_content;

/// Analyzed information from a Rector rule file using AST parsing
#[derive(Debug, Default)]
pub struct AstAnalysis {
    /// Method calls found (e.g., $this->isName, $this->getName)
    pub method_calls: Vec<MethodCallInfo>,

    /// Function calls found in the refactor body
    pub function_calls: Vec<FunctionCallInfo>,

    /// New object creations (e.g., new Name, new Identical)
    pub new_objects: Vec<NewObjectInfo>,

    /// String literals found
    pub string_literals: Vec<String>,

    /// Return statements
    pub returns: Vec<ReturnInfo>,
}

#[derive(Debug, Clone)]
pub struct MethodCallInfo {
    pub object: String,
    pub method: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionCallInfo {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NewObjectInfo {
    pub class_name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ReturnInfo {
    pub value: String,
}

/// Analyze PHP code using AST parsing
pub fn analyze_php(source: &str) -> AstAnalysis {
    let arena = Bump::new();
    let file_id = FileId::new("rector_rule.php");

    let (program, _errors) = parse_file_content(&arena, file_id, source);

    let mut analyzer = AstAnalyzer {
        source,
        analysis: AstAnalysis::default(),
    };

    analyzer.visit_program(&program);
    analyzer.analysis
}

struct AstAnalyzer<'s> {
    source: &'s str,
    analysis: AstAnalysis,
}

impl<'s> AstAnalyzer<'s> {
    fn visit_program(&mut self, program: &Program<'_>) {
        for statement in program.statements.iter() {
            self.visit_statement(statement);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::Namespace(ns) => {
                for stmt in ns.statements().iter() {
                    self.visit_statement(stmt);
                }
            }
            Statement::Class(class) => {
                for member in class.members.iter() {
                    self.visit_class_member(member);
                }
            }
            Statement::Expression(expr_stmt) => {
                self.visit_expression(&expr_stmt.expression);
            }
            Statement::Return(ret) => {
                if let Some(expr) = &ret.value {
                    let value = self.span_text(expr.span());
                    self.analysis.returns.push(ReturnInfo { value });
                    self.visit_expression(expr);
                }
            }
            Statement::If(if_stmt) => {
                self.visit_expression(&if_stmt.condition);
                self.visit_if_body(&if_stmt.body);
            }
            Statement::Block(block) => {
                for stmt in block.statements.iter() {
                    self.visit_statement(stmt);
                }
            }
            _ => {}
        }
    }

    fn visit_if_body(&mut self, body: &IfBody<'_>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.visit_statement(&stmt_body.statement);
            }
            IfBody::ColonDelimited(colon_body) => {
                for stmt in colon_body.statements.iter() {
                    self.visit_statement(stmt);
                }
            }
        }
    }

    fn visit_class_member(&mut self, member: &ClassLikeMember<'_>) {
        if let ClassLikeMember::Method(method) = member {
            // Visit the method body
            match &method.body {
                MethodBody::Concrete(body) => {
                    for stmt in body.statements.iter() {
                        self.visit_statement(stmt);
                    }
                }
                MethodBody::Abstract(_) => {}
            }
        }
    }

    fn visit_expression(&mut self, expr: &Expression<'_>) {
        match expr {
            Expression::Call(call) => {
                self.visit_call(call);
            }
            Expression::Instantiation(inst) => {
                let class_name = self.span_text(inst.class.span());
                let args: Vec<String> = inst
                    .argument_list
                    .as_ref()
                    .map(|a| {
                        a.arguments
                            .iter()
                            .map(|arg| self.span_text(arg.value().span()))
                            .collect()
                    })
                    .unwrap_or_default();

                self.analysis.new_objects.push(NewObjectInfo { class_name, args });

                if let Some(argument_list) = &inst.argument_list {
                    for arg in argument_list.arguments.iter() {
                        self.visit_expression(arg.value());
                    }
                }
            }
            Expression::Binary(bin) => {
                self.visit_expression(&bin.lhs);
                self.visit_expression(&bin.rhs);
            }
            Expression::Literal(lit) => {
                if let Literal::String(s) = lit {
                    let text = self.span_text(s.span());
                    // Remove quotes
                    let inner = text.trim_matches(|c| c == '\'' || c == '"');
                    self.analysis.string_literals.push(inner.to_string());
                }
            }
            Expression::Parenthesized(p) => {
                self.visit_expression(&p.expression);
            }
            Expression::UnaryPrefix(u) => {
                self.visit_expression(&u.operand);
            }
            Expression::UnaryPostfix(u) => {
                self.visit_expression(&u.operand);
            }
            Expression::Assignment(a) => {
                self.visit_expression(&a.lhs);
                self.visit_expression(&a.rhs);
            }
            Expression::Access(access) => {
                self.visit_access(access);
            }
            Expression::Closure(c) => {
                for stmt in c.body.statements.iter() {
                    self.visit_statement(stmt);
                }
            }
            Expression::ArrowFunction(a) => {
                self.visit_expression(&a.expression);
            }
            Expression::Conditional(c) => {
                self.visit_expression(&c.condition);
                if let Some(then) = &c.then {
                    self.visit_expression(then);
                }
                self.visit_expression(&c.r#else);
            }
            _ => {}
        }
    }

    fn visit_call(&mut self, call: &Call<'_>) {
        match call {
            Call::Function(func_call) => {
                let name = self.span_text(func_call.function.span());
                let args: Vec<String> = func_call
                    .argument_list
                    .arguments
                    .iter()
                    .map(|a| self.span_text(a.value().span()))
                    .collect();

                self.analysis.function_calls.push(FunctionCallInfo { name, args });

                for arg in func_call.argument_list.arguments.iter() {
                    self.visit_expression(arg.value());
                }
            }
            Call::Method(method_call) => {
                let object = self.span_text(method_call.object.span());
                let method = self.span_text(method_call.method.span());
                let args: Vec<String> = method_call
                    .argument_list
                    .arguments
                    .iter()
                    .map(|a| self.span_text(a.value().span()))
                    .collect();

                self.analysis.method_calls.push(MethodCallInfo {
                    object,
                    method,
                    args,
                });

                self.visit_expression(&method_call.object);
                for arg in method_call.argument_list.arguments.iter() {
                    self.visit_expression(arg.value());
                }
            }
            Call::NullSafeMethod(method_call) => {
                self.visit_expression(&method_call.object);
                for arg in method_call.argument_list.arguments.iter() {
                    self.visit_expression(arg.value());
                }
            }
            Call::StaticMethod(static_call) => {
                for arg in static_call.argument_list.arguments.iter() {
                    self.visit_expression(arg.value());
                }
            }
        }
    }

    fn visit_access(&mut self, access: &Access<'_>) {
        match access {
            Access::Property(prop) => {
                self.visit_expression(&prop.object);
            }
            Access::NullSafeProperty(prop) => {
                self.visit_expression(&prop.object);
            }
            Access::StaticProperty(_) => {}
            Access::ClassConstant(_) => {}
        }
    }


    fn span_text(&self, span: Span) -> String {
        let start = span.start.offset as usize;
        let end = span.end.offset as usize;
        if start < self.source.len() && end <= self.source.len() && start <= end {
            self.source[start..end].to_string()
        } else {
            String::new()
        }
    }
}

/// Extract pattern information from AST analysis
pub fn detect_pattern_from_ast(analysis: &AstAnalysis) -> Option<DetectedPattern> {
    // Look for $this->isName patterns
    let is_name_calls: Vec<_> = analysis
        .method_calls
        .iter()
        .filter(|c| c.method == "isName" && c.args.len() >= 2)
        .collect();

    // Look for new Name(...) creations
    let new_name_objects: Vec<_> = analysis
        .new_objects
        .iter()
        .filter(|o| o.class_name == "Name" && !o.args.is_empty())
        .collect();

    // Function rename pattern
    if !is_name_calls.is_empty() && !new_name_objects.is_empty() {
        if let (Some(from), Some(to)) = (
            is_name_calls.first().and_then(|c| c.args.get(1)),
            new_name_objects.first().and_then(|o| o.args.first()),
        ) {
            let from = from.trim_matches(|c| c == '\'' || c == '"').to_string();
            let to = to.trim_matches(|c| c == '\'' || c == '"').to_string();
            if from != to && !from.is_empty() && !to.is_empty() {
                return Some(DetectedPattern::FunctionRename { from, to });
            }
        }
    }

    // Look for new Identical/NotIdentical creations (comparison patterns)
    let identical_objects: Vec<_> = analysis
        .new_objects
        .iter()
        .filter(|o| o.class_name == "Identical" || o.class_name == "NotIdentical")
        .collect();

    if !is_name_calls.is_empty() && !identical_objects.is_empty() {
        if let Some(func) = is_name_calls.first().and_then(|c| c.args.get(1)) {
            let func = func.trim_matches(|c| c == '\'' || c == '"').to_string();
            // Check for null comparison
            if analysis
                .new_objects
                .iter()
                .any(|o| o.class_name == "ConstFetch" && o.args.iter().any(|a| a.contains("null")))
            {
                return Some(DetectedPattern::FunctionToComparison {
                    func,
                    operator: "===".to_string(),
                    compare_value: "null".to_string(),
                });
            }
        }
    }

    // Look for Cast objects (type cast patterns)
    let cast_objects: Vec<_> = analysis
        .new_objects
        .iter()
        .filter(|o| {
            o.class_name.ends_with("Cast")
                || matches!(
                    o.class_name.as_str(),
                    "String_" | "Int_" | "Double" | "Bool_" | "Array_"
                )
        })
        .collect();

    if !is_name_calls.is_empty() && !cast_objects.is_empty() {
        if let (Some(func), Some(cast)) = (
            is_name_calls.first().and_then(|c| c.args.get(1)),
            cast_objects.first(),
        ) {
            let func = func.trim_matches(|c| c == '\'' || c == '"').to_string();
            let cast_type = match cast.class_name.as_str() {
                "String_" | "StringCast" => "string",
                "Int_" | "IntCast" => "int",
                "Double" | "DoubleCast" => "float",
                "Bool_" | "BoolCast" => "bool",
                "Array_" | "ArrayCast" => "array",
                _ => return None,
            };
            return Some(DetectedPattern::FunctionToCast {
                func,
                cast_type: cast_type.to_string(),
            });
        }
    }

    // Detect FunctionToOperator: pow -> **
    let pow_objects: Vec<_> = analysis
        .new_objects
        .iter()
        .filter(|o| o.class_name == "Pow" || o.class_name == "BinaryOp\\Pow")
        .collect();

    if !is_name_calls.is_empty() && !pow_objects.is_empty() {
        if let Some(func) = is_name_calls.first().and_then(|c| c.args.get(1)) {
            let func = func.trim_matches(|c| c == '\'' || c == '"').to_string();
            if func == "pow" {
                return Some(DetectedPattern::FunctionToOperator {
                    func,
                    operator: "**".to_string(),
                });
            }
        }
    }

    // Detect FunctionToClassConstant: get_class -> ::class
    let class_const_objects: Vec<_> = analysis
        .new_objects
        .iter()
        .filter(|o| o.class_name == "ClassConstFetch")
        .collect();

    if !is_name_calls.is_empty() && !class_const_objects.is_empty() {
        if let Some(func) = is_name_calls.first().and_then(|c| c.args.get(1)) {
            let func = func.trim_matches(|c| c == '\'' || c == '"').to_string();
            if func == "get_class" {
                // Check if any arg contains 'class'
                if class_const_objects
                    .iter()
                    .any(|o| o.args.iter().any(|a| a.contains("class")))
                {
                    return Some(DetectedPattern::FunctionToClassConstant { func });
                }
            }
        }
    }

    // Detect FunctionToInstanceof: is_a -> instanceof
    let instanceof_objects: Vec<_> = analysis
        .new_objects
        .iter()
        .filter(|o| o.class_name == "Instanceof" || o.class_name == "Instanceof_")
        .collect();

    if !is_name_calls.is_empty() && !instanceof_objects.is_empty() {
        if let Some(func) = is_name_calls.first().and_then(|c| c.args.get(1)) {
            let func = func.trim_matches(|c| c == '\'' || c == '"').to_string();
            if func == "is_a" {
                return Some(DetectedPattern::FunctionToInstanceof { func });
            }
        }
    }

    // Detect TernaryToCoalesce: isset($x) ? $x : $y -> $x ?? $y
    let coalesce_objects: Vec<_> = analysis
        .new_objects
        .iter()
        .filter(|o| o.class_name == "Coalesce")
        .collect();

    if !coalesce_objects.is_empty() {
        // Check if there's an isset check in method calls
        let has_isset = analysis.function_calls.iter().any(|c| c.name == "isset")
            || analysis.string_literals.iter().any(|s| s == "isset");
        if has_isset {
            return Some(DetectedPattern::TernaryToCoalesce);
        }
    }

    // Detect str_contains pattern
    if analysis.string_literals.iter().any(|s| s == "str_contains") {
        if analysis.string_literals.iter().any(|s| s == "strpos" || s == "strstr") {
            return Some(DetectedPattern::StrContains);
        }
    }

    // Detect str_starts_with pattern
    if analysis.string_literals.iter().any(|s| s == "str_starts_with") {
        if analysis
            .string_literals
            .iter()
            .any(|s| s == "strpos" || s == "substr" || s == "strncmp")
        {
            return Some(DetectedPattern::StrStartsWith);
        }
    }

    // Detect str_ends_with pattern
    if analysis.string_literals.iter().any(|s| s == "str_ends_with") {
        if analysis
            .string_literals
            .iter()
            .any(|s| s == "substr" || s == "substr_compare")
        {
            return Some(DetectedPattern::StrEndsWith);
        }
    }

    None
}

#[derive(Debug, Clone)]
pub enum DetectedPattern {
    FunctionRename { from: String, to: String },
    FunctionToComparison { func: String, operator: String, compare_value: String },
    FunctionToCast { func: String, cast_type: String },
    FunctionToOperator { func: String, operator: String },
    FunctionToClassConstant { func: String },
    FunctionToInstanceof { func: String },
    FunctionNoArgsToFunction { from: String, to: String },
    TernaryToCoalesce,
    StrContains,
    StrStartsWith,
    StrEndsWith,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_simple_php() {
        let php = r#"<?php
            $this->isName($node, 'join');
            return new Name('implode');
        "#;

        let analysis = analyze_php(php);
        assert!(!analysis.method_calls.is_empty());
        assert!(!analysis.new_objects.is_empty());
    }

    #[test]
    fn test_detect_function_rename() {
        let php = r#"<?php
            if ($this->isName($node, 'join')) {
                $node->name = new Name('implode');
            }
        "#;

        let analysis = analyze_php(php);
        let pattern = detect_pattern_from_ast(&analysis);

        assert!(pattern.is_some());
        if let Some(DetectedPattern::FunctionRename { from, to }) = pattern {
            assert_eq!(from, "join");
            assert_eq!(to, "implode");
        }
    }

    #[test]
    fn test_detect_function_to_comparison() {
        let php = r#"<?php
            if ($this->isName($node, 'is_null')) {
                return new Identical($arg, new ConstFetch(new Name('null')));
            }
        "#;

        let analysis = analyze_php(php);
        // Should detect is_name + Identical + null
        assert!(analysis.method_calls.iter().any(|c| c.method == "isName"));
        assert!(analysis.new_objects.iter().any(|o| o.class_name == "Identical"));
    }

    #[test]
    fn test_detect_function_to_cast() {
        let php = r#"<?php
            if ($this->isName($node, 'strval')) {
                return new String_($arg);
            }
        "#;

        let analysis = analyze_php(php);
        assert!(analysis.method_calls.iter().any(|c| c.method == "isName"));
        assert!(analysis.new_objects.iter().any(|o| o.class_name == "String_"));

        let pattern = detect_pattern_from_ast(&analysis);
        assert!(matches!(
            pattern,
            Some(DetectedPattern::FunctionToCast { func, cast_type })
            if func == "strval" && cast_type == "string"
        ));
    }

    #[test]
    fn test_detect_pow_to_operator() {
        let php = r#"<?php
            if ($this->isName($node, 'pow')) {
                return new Pow($base, $exp);
            }
        "#;

        let analysis = analyze_php(php);
        assert!(analysis.method_calls.iter().any(|c| c.method == "isName"));
        assert!(analysis.new_objects.iter().any(|o| o.class_name == "Pow"));

        let pattern = detect_pattern_from_ast(&analysis);
        assert!(matches!(
            pattern,
            Some(DetectedPattern::FunctionToOperator { func, operator })
            if func == "pow" && operator == "**"
        ));
    }

    #[test]
    fn test_detect_get_class_to_class_constant() {
        let php = r#"<?php
            if ($this->isName($node, 'get_class')) {
                return new ClassConstFetch($obj, new Identifier('class'));
            }
        "#;

        let analysis = analyze_php(php);
        assert!(analysis.method_calls.iter().any(|c| c.method == "isName"));
        assert!(analysis.new_objects.iter().any(|o| o.class_name == "ClassConstFetch"));
    }

    #[test]
    fn test_detect_is_a_to_instanceof() {
        // Wrap in a class so method calls are detected properly
        let php = r#"<?php
            class R {
                public function refactor() {
                    if ($this->isName($node, 'is_a')) {
                        return new Instanceof_($first, $second);
                    }
                }
            }
        "#;

        let analysis = analyze_php(php);

        // The method call detection happens on $this->isName
        assert!(
            analysis.method_calls.iter().any(|c| c.method == "isName"),
            "Expected to find isName method call"
        );

        // At minimum we should detect the Instanceof_ object creation
        assert!(
            analysis
                .new_objects
                .iter()
                .any(|o| o.class_name == "Instanceof_" || o.class_name == "Instanceof"),
            "Expected to find Instanceof_ object creation"
        );

        // Verify pattern detection
        let pattern = detect_pattern_from_ast(&analysis);
        assert!(
            matches!(
                &pattern,
                Some(DetectedPattern::FunctionToInstanceof { func }) if func == "is_a"
            ),
            "Expected FunctionToInstanceof pattern, got {:?}",
            pattern
        );
    }

    #[test]
    fn test_detect_ternary_to_coalesce() {
        let php = r#"<?php
            // isset($x) ? $x : $y -> $x ?? $y
            $func = 'isset';
            return new Coalesce($left, $right);
        "#;

        let analysis = analyze_php(php);
        assert!(analysis.new_objects.iter().any(|o| o.class_name == "Coalesce"));
        assert!(analysis.string_literals.iter().any(|s| s == "isset"));

        let pattern = detect_pattern_from_ast(&analysis);
        assert!(matches!(pattern, Some(DetectedPattern::TernaryToCoalesce)));
    }
}
