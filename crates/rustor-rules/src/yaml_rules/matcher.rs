//! Pattern matching engine for YAML rules
//!
//! Matches YAML patterns against mago-syntax AST nodes and captures
//! variable bindings for use in replacements.

use mago_span::{HasSpan, Span};
use mago_syntax::ast::*;
use std::collections::HashMap;

use super::schema::{
    ArgPattern, CaptureOrPattern, MatchPattern, NodePattern, StringOrCapture,
};

/// Captured bindings from a successful pattern match
#[derive(Debug, Clone, Default)]
pub struct CapturedBindings {
    /// Map from capture variable name to captured source text
    bindings: HashMap<String, CapturedValue>,
}

/// A captured value with its source location
#[derive(Debug, Clone)]
pub struct CapturedValue {
    /// The captured source text
    pub text: String,
    /// The span in the original source
    pub span: Span,
}

impl CapturedBindings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding
    pub fn insert(&mut self, name: String, text: String, span: Span) {
        self.bindings.insert(name, CapturedValue { text, span });
    }

    /// Get a binding value
    pub fn get(&self, name: &str) -> Option<&CapturedValue> {
        // Handle both with and without $ prefix
        self.bindings
            .get(name)
            .or_else(|| self.bindings.get(name.strip_prefix('$').unwrap_or(name)))
    }

    /// Get the text for a binding
    pub fn get_text(&self, name: &str) -> Option<&str> {
        self.get(name).map(|v| v.text.as_str())
    }

    /// Check if a binding exists
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Merge another set of bindings into this one
    pub fn merge(&mut self, other: CapturedBindings) {
        self.bindings.extend(other.bindings);
    }

    /// Get all bindings
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CapturedValue)> {
        self.bindings.iter()
    }
}

/// Pattern matcher for YAML rules
pub struct PatternMatcher<'s> {
    source: &'s str,
}

impl<'s> PatternMatcher<'s> {
    pub fn new(source: &'s str) -> Self {
        Self { source }
    }

    /// Get source text for a span
    fn span_text(&self, span: &Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    /// Match a pattern against an expression
    pub fn match_expression<'a>(
        &self,
        pattern: &MatchPattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        match pattern {
            MatchPattern::Node(node_pattern) => self.match_node_pattern(node_pattern, expr),
            MatchPattern::Any { any } => {
                // Try each pattern in order, return first match
                for p in any {
                    if let Some(bindings) = self.match_expression(p, expr) {
                        return Some(bindings);
                    }
                }
                None
            }
            MatchPattern::All { all } => {
                // All patterns must match, merge bindings
                let mut combined = CapturedBindings::new();
                for p in all {
                    match self.match_expression(p, expr) {
                        Some(bindings) => combined.merge(bindings),
                        None => return None,
                    }
                }
                Some(combined)
            }
        }
    }

    /// Match a node pattern against an expression
    fn match_node_pattern<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        match pattern.node.as_str() {
            "FuncCall" => self.match_func_call(pattern, expr),
            "MethodCall" => self.match_method_call(pattern, expr),
            "StaticCall" => self.match_static_call(pattern, expr),
            "BinaryOp" => self.match_binary_op(pattern, expr),
            "Ternary" | "Conditional" => self.match_ternary(pattern, expr),
            "Array" => self.match_array(pattern, expr),
            "LiteralFalse" => self.match_literal_false(expr),
            "LiteralTrue" => self.match_literal_true(expr),
            "LiteralNull" | "Null" => self.match_literal_null(expr),
            "LiteralInt" | "Integer" => self.match_literal_int(pattern, expr),
            "LiteralString" | "String" => self.match_literal_string(pattern, expr),
            "Variable" => self.match_variable(pattern, expr),
            "PropertyFetch" => self.match_property_fetch(pattern, expr),
            "ArrayAccess" => self.match_array_access(pattern, expr),
            "UnaryOp" | "BooleanNot" => self.match_unary_op(pattern, expr),
            "Isset" => self.match_isset(pattern, expr),
            "Empty" => self.match_empty(pattern, expr),
            _ => None,
        }
    }

    /// Match a function call
    fn match_func_call<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Call(Call::Function(call)) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match function name - get it from the function expression
        if let Some(name_pattern) = &pattern.name {
            let func_span = call.function.span();
            let func_name = self.span_text(&func_span);
            match name_pattern {
                StringOrCapture::Literal(expected) => {
                    if !func_name.eq_ignore_ascii_case(expected) {
                        return None;
                    }
                }
                StringOrCapture::Capture { capture } => {
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    bindings.insert(name.to_string(), func_name.to_string(), func_span);
                }
            }
        }

        // Match arguments
        let args: Vec<_> = call.argument_list.arguments.iter().collect();
        if let Some(arg_bindings) = self.match_arguments(&pattern.args, &args) {
            bindings.merge(arg_bindings);
        } else if !pattern.args.is_empty() {
            return None;
        }

        Some(bindings)
    }

    /// Match a method call ($obj->method())
    fn match_method_call<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Call(Call::Method(call)) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match object
        if let Some(obj_pattern) = &pattern.object {
            if let Some(obj_bindings) = self.match_capture_or_pattern(obj_pattern, &call.object) {
                bindings.merge(obj_bindings);
            } else {
                return None;
            }
        }

        // Match method name from selector
        if let Some(method_pattern) = &pattern.method {
            let method_span = call.method.span();
            let method_name = self.span_text(&method_span);
            match method_pattern {
                StringOrCapture::Literal(expected) => {
                    if !method_name.eq_ignore_ascii_case(expected) {
                        return None;
                    }
                }
                StringOrCapture::Capture { capture } => {
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    bindings.insert(name.to_string(), method_name.to_string(), method_span);
                }
            }
        }

        // Match arguments
        let args: Vec<_> = call.argument_list.arguments.iter().collect();
        if let Some(arg_bindings) = self.match_arguments(&pattern.args, &args) {
            bindings.merge(arg_bindings);
        } else if !pattern.args.is_empty() {
            return None;
        }

        Some(bindings)
    }

    /// Match a static method call (Class::method())
    fn match_static_call<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Call(Call::StaticMethod(call)) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match class name
        if let Some(class_pattern) = &pattern.class {
            let class_span = call.class.span();
            let class_name = self.span_text(&class_span);
            match class_pattern {
                StringOrCapture::Literal(expected) => {
                    if !class_name.eq_ignore_ascii_case(expected) {
                        return None;
                    }
                }
                StringOrCapture::Capture { capture } => {
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    bindings.insert(name.to_string(), class_name.to_string(), class_span);
                }
            }
        }

        // Match method name from selector
        if let Some(method_pattern) = &pattern.method {
            let method_span = call.method.span();
            let method_name = self.span_text(&method_span);
            match method_pattern {
                StringOrCapture::Literal(expected) => {
                    if !method_name.eq_ignore_ascii_case(expected) {
                        return None;
                    }
                }
                StringOrCapture::Capture { capture } => {
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    bindings.insert(name.to_string(), method_name.to_string(), method_span);
                }
            }
        }

        // Match arguments
        let args: Vec<_> = call.argument_list.arguments.iter().collect();
        if let Some(arg_bindings) = self.match_arguments(&pattern.args, &args) {
            bindings.merge(arg_bindings);
        } else if !pattern.args.is_empty() {
            return None;
        }

        Some(bindings)
    }

    /// Match a binary operation
    fn match_binary_op<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Binary(binary) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match operator
        if let Some(op_pattern) = &pattern.operator {
            let op_str = self.binary_op_to_string(&binary.operator);
            if op_str != op_pattern {
                return None;
            }
        }

        // Match left operand
        if let Some(left_pattern) = &pattern.left {
            if let Some(left_bindings) = self.match_capture_or_pattern(left_pattern, &binary.lhs) {
                bindings.merge(left_bindings);
            } else {
                return None;
            }
        }

        // Match right operand
        if let Some(right_pattern) = &pattern.right {
            if let Some(right_bindings) = self.match_capture_or_pattern(right_pattern, &binary.rhs) {
                bindings.merge(right_bindings);
            } else {
                return None;
            }
        }

        Some(bindings)
    }

    /// Match a ternary expression (conditional)
    fn match_ternary<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Conditional(ternary) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match condition
        if let Some(cond_pattern) = &pattern.condition {
            if let Some(cond_bindings) = self.match_capture_or_pattern(cond_pattern, &ternary.condition) {
                bindings.merge(cond_bindings);
            } else {
                return None;
            }
        }

        // Match then branch
        if let Some(then_pattern) = &pattern.then {
            if let Some(then_expr) = &ternary.then {
                if let Some(then_bindings) = self.match_capture_or_pattern(then_pattern, then_expr) {
                    bindings.merge(then_bindings);
                } else {
                    return None;
                }
            } else {
                // Elvis operator (?:) - then is None
                // For same_as patterns, the "then" is the condition itself
                if let CaptureOrPattern::SameAs { same_as } = then_pattern.as_ref() {
                    // Check if condition was captured with this name
                    let name = same_as.strip_prefix('$').unwrap_or(same_as);
                    if !bindings.contains(name) {
                        return None;
                    }
                } else {
                    return None;
                }
            }
        }

        // Match else branch
        if let Some(else_pattern) = &pattern.else_branch {
            if let Some(else_bindings) = self.match_capture_or_pattern(else_pattern, &ternary.r#else) {
                bindings.merge(else_bindings);
            } else {
                return None;
            }
        }

        Some(bindings)
    }

    /// Match an array literal
    fn match_array<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let bindings = CapturedBindings::new();

        // Match syntax type
        if let Some(syntax) = &pattern.syntax {
            let is_short = matches!(expr, Expression::Array(_));
            let is_long = matches!(expr, Expression::LegacyArray(_));

            match syntax.as_str() {
                "long" if !is_long => return None,
                "short" if !is_short => return None,
                _ => {}
            }
        } else {
            // Must be either Array or LegacyArray
            if !matches!(expr, Expression::Array(_) | Expression::LegacyArray(_)) {
                return None;
            }
        }

        // TODO: Match array items
        Some(bindings)
    }

    /// Match a capture or pattern against an expression
    fn match_capture_or_pattern<'a>(
        &self,
        pattern: &CaptureOrPattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        match pattern {
            CaptureOrPattern::Capture(name) | CaptureOrPattern::CaptureExplicit { capture: name } => {
                let mut bindings = CapturedBindings::new();
                let clean_name = name.strip_prefix('$').unwrap_or(name);
                let span = expr.span();
                let text = self.span_text(&span);
                bindings.insert(clean_name.to_string(), text.to_string(), span);
                Some(bindings)
            }
            CaptureOrPattern::SameAs { same_as } => {
                // This requires the referenced capture to already exist
                // Will be validated during matching of parent pattern
                let mut bindings = CapturedBindings::new();
                let clean_name = same_as.strip_prefix('$').unwrap_or(same_as);
                let span = expr.span();
                let text = self.span_text(&span);
                bindings.insert(format!("_same_as_{}", clean_name), text.to_string(), span);
                Some(bindings)
            }
            CaptureOrPattern::Pattern(node_pattern) => {
                self.match_node_pattern(node_pattern, expr)
            }
        }
    }

    /// Match function arguments
    fn match_arguments<'a>(
        &self,
        patterns: &[ArgPattern],
        args: &[&Argument<'a>],
    ) -> Option<CapturedBindings> {
        let mut bindings = CapturedBindings::new();
        let mut arg_idx = 0;

        for pattern in patterns {
            match pattern {
                ArgPattern::Capture { capture } => {
                    if arg_idx >= args.len() {
                        return None;
                    }
                    let arg = args[arg_idx];
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    let arg_span = arg.value().span();
                    let text = self.span_text(&arg_span);
                    bindings.insert(name.to_string(), text.to_string(), arg_span);
                    arg_idx += 1;
                }
                ArgPattern::Literal { literal } => {
                    if arg_idx >= args.len() {
                        return None;
                    }
                    let arg = args[arg_idx];
                    if !self.match_literal_value(literal, arg.value()) {
                        return None;
                    }
                    arg_idx += 1;
                }
                ArgPattern::Spread { capture } => {
                    // Capture all remaining arguments
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    let remaining: Vec<_> = args[arg_idx..]
                        .iter()
                        .map(|a| self.span_text(&a.value().span()))
                        .collect();
                    let text = remaining.join(", ");
                    // Use the span from first remaining arg to last
                    if let Some(first) = args.get(arg_idx) {
                        if let Some(last) = args.last() {
                            let first_span = first.value().span();
                            let last_span = last.value().span();
                            let span = Span {
                                file_id: first_span.file_id,
                                start: first_span.start,
                                end: last_span.end,
                            };
                            bindings.insert(name.to_string(), text, span);
                        }
                    }
                    break; // Spread consumes all remaining
                }
                ArgPattern::NoMore { no_more: true } => {
                    if arg_idx < args.len() {
                        return None;
                    }
                }
                ArgPattern::NoMore { no_more: false } => {}
                ArgPattern::Optional { capture, optional: true } => {
                    if arg_idx < args.len() {
                        let arg = args[arg_idx];
                        let name = capture.strip_prefix('$').unwrap_or(capture);
                        let arg_span = arg.value().span();
                        let text = self.span_text(&arg_span);
                        bindings.insert(name.to_string(), text.to_string(), arg_span);
                        arg_idx += 1;
                    }
                }
                ArgPattern::Optional { optional: false, .. } => {}
            }
        }

        Some(bindings)
    }

    /// Match a literal value
    fn match_literal_value<'a>(
        &self,
        pattern: &super::schema::ArgValue,
        expr: &Expression<'a>,
    ) -> bool {
        match pattern {
            super::schema::ArgValue::Bool(b) => {
                match expr {
                    Expression::Literal(Literal::True(_)) => *b,
                    Expression::Literal(Literal::False(_)) => !*b,
                    _ => false,
                }
            }
            super::schema::ArgValue::Int(n) => {
                if let Expression::Literal(Literal::Integer(lit)) = expr {
                    let text = self.span_text(&lit.span);
                    text.parse::<i64>().ok() == Some(*n)
                } else {
                    false
                }
            }
            super::schema::ArgValue::Float(f) => {
                if let Expression::Literal(Literal::Float(lit)) = expr {
                    let text = self.span_text(&lit.span);
                    text.parse::<f64>().ok() == Some(*f)
                } else {
                    false
                }
            }
            super::schema::ArgValue::String(s) => {
                if let Expression::Literal(Literal::String(lit)) = expr {
                    let text = self.span_text(&lit.span());
                    // Strip quotes and compare
                    let inner = text.trim_matches(|c| c == '"' || c == '\'');
                    inner == s
                } else {
                    false
                }
            }
            super::schema::ArgValue::Null => {
                matches!(expr, Expression::Literal(Literal::Null(_)))
            }
        }
    }

    /// Match literal false
    fn match_literal_false<'a>(&self, expr: &Expression<'a>) -> Option<CapturedBindings> {
        if matches!(expr, Expression::Literal(Literal::False(_))) {
            Some(CapturedBindings::new())
        } else {
            None
        }
    }

    /// Match literal true
    fn match_literal_true<'a>(&self, expr: &Expression<'a>) -> Option<CapturedBindings> {
        if matches!(expr, Expression::Literal(Literal::True(_))) {
            Some(CapturedBindings::new())
        } else {
            None
        }
    }

    /// Match literal null
    fn match_literal_null<'a>(&self, expr: &Expression<'a>) -> Option<CapturedBindings> {
        if matches!(expr, Expression::Literal(Literal::Null(_))) {
            Some(CapturedBindings::new())
        } else {
            None
        }
    }

    /// Match literal int
    fn match_literal_int<'a>(
        &self,
        _pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        if matches!(expr, Expression::Literal(Literal::Integer(_))) {
            Some(CapturedBindings::new())
        } else {
            None
        }
    }

    /// Match literal string
    fn match_literal_string<'a>(
        &self,
        _pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        if matches!(expr, Expression::Literal(Literal::String(_))) {
            Some(CapturedBindings::new())
        } else {
            None
        }
    }

    /// Match a variable
    fn match_variable<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Variable(var) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match variable name if specified
        if let Some(name_pattern) = &pattern.name {
            let var_span = var.span();
            let var_name = self.span_text(&var_span);
            match name_pattern {
                StringOrCapture::Literal(expected) => {
                    if var_name != expected {
                        return None;
                    }
                }
                StringOrCapture::Capture { capture } => {
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    bindings.insert(name.to_string(), var_name.to_string(), var_span);
                }
            }
        }

        Some(bindings)
    }

    /// Match a property fetch ($obj->prop)
    fn match_property_fetch<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Access(Access::Property(access)) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match object
        if let Some(obj_pattern) = &pattern.object {
            if let Some(obj_bindings) = self.match_capture_or_pattern(obj_pattern, &access.object) {
                bindings.merge(obj_bindings);
            } else {
                return None;
            }
        }

        Some(bindings)
    }

    /// Match array access ($arr['key'] or $arr[0])
    fn match_array_access<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::ArrayAccess(access) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Match array
        if let Some(obj_pattern) = &pattern.object {
            if let Some(obj_bindings) = self.match_capture_or_pattern(obj_pattern, access.array) {
                bindings.merge(obj_bindings);
            } else {
                return None;
            }
        }

        Some(bindings)
    }

    /// Match unary operation
    fn match_unary_op<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::UnaryPrefix(unary) = expr else {
            return None;
        };

        // For BooleanNot, check operator
        if pattern.node == "BooleanNot" {
            if !matches!(unary.operator, UnaryPrefixOperator::Not(_)) {
                return None;
            }
        }

        let mut bindings = CapturedBindings::new();

        // Capture the operand
        if let Some(left_pattern) = &pattern.left {
            if let Some(left_bindings) = self.match_capture_or_pattern(left_pattern, &unary.operand) {
                bindings.merge(left_bindings);
            } else {
                return None;
            }
        }

        Some(bindings)
    }

    /// Match isset() construct
    fn match_isset<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Construct(Construct::Isset(isset)) = expr else {
            return None;
        };

        // Match arguments
        let args: Vec<_> = isset.values.iter().collect();
        let mut bindings = CapturedBindings::new();

        for (i, arg_pattern) in pattern.args.iter().enumerate() {
            if let ArgPattern::Capture { capture } = arg_pattern {
                if i < args.len() {
                    let name = capture.strip_prefix('$').unwrap_or(capture);
                    let arg_span = args[i].span();
                    let text = self.span_text(&arg_span);
                    bindings.insert(name.to_string(), text.to_string(), arg_span);
                }
            }
        }

        Some(bindings)
    }

    /// Match empty() construct
    fn match_empty<'a>(
        &self,
        pattern: &NodePattern,
        expr: &Expression<'a>,
    ) -> Option<CapturedBindings> {
        let Expression::Construct(Construct::Empty(empty)) = expr else {
            return None;
        };

        let mut bindings = CapturedBindings::new();

        // Capture the single argument
        if let Some(ArgPattern::Capture { capture }) = pattern.args.first() {
            let name = capture.strip_prefix('$').unwrap_or(capture);
            let value_span = empty.value.span();
            let text = self.span_text(&value_span);
            bindings.insert(name.to_string(), text.to_string(), value_span);
        }

        Some(bindings)
    }

    /// Convert binary operator to string representation
    fn binary_op_to_string(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Identical(_) => "===",
            BinaryOperator::NotIdentical(_) => "!==",
            BinaryOperator::Equal(_) => "==",
            BinaryOperator::NotEqual(_) => "!=",
            BinaryOperator::LessThan(_) => "<",
            BinaryOperator::GreaterThan(_) => ">",
            BinaryOperator::LessThanOrEqual(_) => "<=",
            BinaryOperator::GreaterThanOrEqual(_) => ">=",
            BinaryOperator::Addition(_) => "+",
            BinaryOperator::Subtraction(_) => "-",
            BinaryOperator::Multiplication(_) => "*",
            BinaryOperator::Division(_) => "/",
            BinaryOperator::Modulo(_) => "%",
            BinaryOperator::Exponentiation(_) => "**",
            BinaryOperator::StringConcat(_) => ".",
            BinaryOperator::And(_) => "&&",
            BinaryOperator::Or(_) => "||",
            BinaryOperator::BitwiseAnd(_) => "&",
            BinaryOperator::BitwiseOr(_) => "|",
            BinaryOperator::BitwiseXor(_) => "^",
            BinaryOperator::LeftShift(_) => "<<",
            BinaryOperator::RightShift(_) => ">>",
            BinaryOperator::NullCoalesce(_) => "??",
            BinaryOperator::Spaceship(_) => "<=>",
            BinaryOperator::LowAnd(_) => "and",
            BinaryOperator::LowOr(_) => "or",
            BinaryOperator::LowXor(_) => "xor",
            BinaryOperator::Instanceof(_) => "instanceof",
            BinaryOperator::AngledNotEqual(_) => "<>",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use mago_database::file::FileId;
    use mago_syntax::parser::parse_file_content;

    fn parse_expr(code: &str) -> (String, Program<'static>) {
        let full_code = format!("<?php {};", code);
        let bump = Box::leak(Box::new(Bump::new()));
        let file_id = FileId::new("test.php");
        let (program, _) = parse_file_content(bump, file_id, &full_code);
        (full_code, program.clone())
    }

    #[test]
    fn test_match_func_call() {
        let (source, program) = parse_expr("is_null($x)");
        let matcher = PatternMatcher::new(&source);

        let pattern = NodePattern {
            node: "FuncCall".to_string(),
            name: Some(StringOrCapture::Literal("is_null".to_string())),
            args: vec![ArgPattern::Capture {
                capture: "$expr".to_string(),
            }],
            class: None,
            method: None,
            object: None,
            operator: None,
            left: None,
            right: None,
            condition: None,
            then: None,
            else_branch: None,
            syntax: None,
            items: None,
        };

        // Get the expression from the parsed program
        if let Some(Statement::Expression(stmt)) = program.statements.first() {
            let bindings = matcher.match_node_pattern(&pattern, &stmt.expression);
            assert!(bindings.is_some());
            let bindings = bindings.unwrap();
            assert!(bindings.contains("expr"));
            assert_eq!(bindings.get_text("expr"), Some("$x"));
        }
    }
}
