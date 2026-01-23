//! Argument type validation (Level 5)
//!
//! Checks that arguments passed to functions/methods match the expected types.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use mago_span::HasSpan;
use mago_syntax::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Check for argument type mismatches
pub struct ArgumentTypeCheck;

impl Check for ArgumentTypeCheck {
    fn id(&self) -> &'static str {
        "argument.type"
    }

    fn description(&self) -> &'static str {
        "Checks that arguments passed to functions/methods match expected types"
    }

    fn level(&self) -> u8 {
        5
    }

    fn check<'a>(&self, program: &Program<'a>, ctx: &CheckContext<'_>) -> Vec<Issue> {
        let mut visitor = ArgumentTypeVisitor {
            source: ctx.source,
            file_path: ctx.file_path.to_path_buf(),
            functions: HashMap::new(),
            methods: HashMap::new(),
            param_types: HashMap::new(),
            variable_types: HashMap::new(),
            current_class: None,
            builtin_functions: ctx.builtin_functions,
            issues: Vec::new(),
        };

        // First pass: collect function/method signatures
        visitor.collect_signatures(program);

        // Second pass: check argument types
        visitor.analyze_program(program);

        visitor.issues
    }
}

/// Information about a function/method parameter
#[derive(Debug, Clone)]
struct ParamInfo {
    name: String,
    type_hint: Option<String>,
    is_nullable: bool,
    #[allow(dead_code)]
    has_default: bool,
}

/// Information about a function/method
#[derive(Debug, Clone)]
struct FunctionInfo {
    name: String,
    params: Vec<ParamInfo>,
}

struct ArgumentTypeVisitor<'s> {
    source: &'s str,
    file_path: PathBuf,
    /// Function signatures: name (lowercase) -> info
    functions: HashMap<String, FunctionInfo>,
    /// Method signatures: "ClassName::methodName" (lowercase) -> info
    methods: HashMap<String, FunctionInfo>,
    /// Parameter types in current function scope
    param_types: HashMap<String, String>,
    /// Variable types from assignments
    variable_types: HashMap<String, String>,
    /// Current class context
    current_class: Option<String>,
    /// Built-in functions
    builtin_functions: &'s [&'static str],
    issues: Vec<Issue>,
}

impl<'s> ArgumentTypeVisitor<'s> {
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }

    fn get_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Collect function and method signatures
    fn collect_signatures<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.collect_from_statement(stmt);
        }
    }

    fn collect_from_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                let name = self.get_span_text(&func.name.span).to_string();
                let info = self.extract_function_info(&name, &func.parameter_list);
                self.functions.insert(name.to_lowercase(), info);
            }
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();

                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        let method_name = self.get_span_text(&method.name.span).to_string();
                        let full_name = format!("{}::{}", class_name, method_name);
                        let info = self.extract_function_info(&full_name, &method.parameter_list);
                        self.methods.insert(full_name.to_lowercase(), info);
                    }
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.collect_from_statement(inner);
                    }
                }
            },
            _ => {}
        }
    }

    fn extract_function_info(
        &self,
        name: &str,
        params: &FunctionLikeParameterList<'_>,
    ) -> FunctionInfo {
        let mut param_infos = Vec::new();

        for param in params.parameters.iter() {
            let param_name = self.get_span_text(&param.variable.span).to_string();
            let (type_hint, is_nullable) = if let Some(hint) = &param.hint {
                (self.extract_type_name(hint), self.is_nullable_hint(hint))
            } else {
                (None, false)
            };

            param_infos.push(ParamInfo {
                name: param_name,
                type_hint,
                is_nullable,
                has_default: param.default_value.is_some(),
            });
        }

        FunctionInfo {
            name: name.to_string(),
            params: param_infos,
        }
    }

    fn extract_type_name(&self, hint: &Hint<'_>) -> Option<String> {
        match hint {
            // Class/interface names
            Hint::Identifier(ident) => Some(self.get_span_text(&ident.span()).to_string()),
            Hint::Nullable(nullable) => self.extract_type_name(&nullable.hint),
            Hint::Parenthesized(p) => self.extract_type_name(&p.hint),
            // Built-in types
            Hint::Integer(_) => Some("int".to_string()),
            Hint::String(_) => Some("string".to_string()),
            Hint::Float(_) => Some("float".to_string()),
            Hint::Bool(_) => Some("bool".to_string()),
            Hint::Array(_) => Some("array".to_string()),
            Hint::Object(_) => Some("object".to_string()),
            Hint::Mixed(_) => Some("mixed".to_string()),
            Hint::Callable(_) => Some("callable".to_string()),
            Hint::Iterable(_) => Some("iterable".to_string()),
            Hint::Void(_) => Some("void".to_string()),
            Hint::Never(_) => Some("never".to_string()),
            Hint::Null(_) => Some("null".to_string()),
            Hint::True(_) => Some("true".to_string()),
            Hint::False(_) => Some("false".to_string()),
            // Union types - return first type for now
            Hint::Union(union) => self.extract_type_name(union.left),
            Hint::Intersection(intersection) => self.extract_type_name(intersection.left),
            _ => None,
        }
    }

    fn is_nullable_hint(&self, hint: &Hint<'_>) -> bool {
        matches!(hint, Hint::Nullable(_))
    }

    fn analyze_program<'a>(&mut self, program: &Program<'a>) {
        for stmt in program.statements.iter() {
            self.analyze_statement(stmt);
        }
    }

    fn analyze_statement<'a>(&mut self, stmt: &Statement<'a>) {
        match stmt {
            Statement::Function(func) => {
                // Track parameter types
                self.param_types.clear();
                for param in func.parameter_list.parameters.iter() {
                    if let Some(hint) = &param.hint {
                        if let Some(type_name) = self.extract_type_name(hint) {
                            let var_name = self.get_span_text(&param.variable.span).to_string();
                            self.param_types.insert(var_name, type_name);
                        }
                    }
                }

                for inner in func.body.statements.iter() {
                    self.analyze_statement(inner);
                }

                self.param_types.clear();
            }
            Statement::Class(class) => {
                let class_name = self.get_span_text(&class.name.span).to_string();
                self.current_class = Some(class_name);

                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            // Track parameter types
                            self.param_types.clear();
                            for param in method.parameter_list.parameters.iter() {
                                if let Some(hint) = &param.hint {
                                    if let Some(type_name) = self.extract_type_name(hint) {
                                        let var_name =
                                            self.get_span_text(&param.variable.span).to_string();
                                        self.param_types.insert(var_name, type_name);
                                    }
                                }
                            }

                            for inner in body.statements.iter() {
                                self.analyze_statement(inner);
                            }

                            self.param_types.clear();
                        }
                    }
                }

                self.current_class = None;
            }
            Statement::Expression(expr_stmt) => {
                // Track variable assignments
                if let Expression::Assignment(assign) = expr_stmt.expression {
                    if let Expression::Variable(Variable::Direct(var)) = assign.lhs {
                        let var_name = self.get_span_text(&var.span).to_string();
                        if let Some(type_name) = self.infer_expression_type(assign.rhs) {
                            self.variable_types.insert(var_name, type_name);
                        }
                    }
                }
                self.check_expression(expr_stmt.expression);
            }
            Statement::If(if_stmt) => {
                self.check_expression(&if_stmt.condition);
                self.analyze_if_body(&if_stmt.body);
            }
            Statement::While(while_stmt) => {
                self.check_expression(&while_stmt.condition);
                self.analyze_while_body(&while_stmt.body);
            }
            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.check_expression(value);
                }
            }
            Statement::Echo(echo) => {
                for value in echo.values.iter() {
                    self.check_expression(value);
                }
            }
            Statement::Block(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
            Statement::Namespace(ns) => match &ns.body {
                NamespaceBody::Implicit(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
                NamespaceBody::BraceDelimited(body) => {
                    for inner in body.statements.iter() {
                        self.analyze_statement(inner);
                    }
                }
            },
            _ => {}
        }
    }

    fn analyze_if_body<'a>(&mut self, body: &IfBody<'a>) {
        match body {
            IfBody::Statement(stmt_body) => {
                self.analyze_statement(stmt_body.statement);
                for else_if in stmt_body.else_if_clauses.iter() {
                    self.check_expression(&else_if.condition);
                    self.analyze_statement(else_if.statement);
                }
                if let Some(else_clause) = &stmt_body.else_clause {
                    self.analyze_statement(else_clause.statement);
                }
            }
            IfBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn analyze_while_body<'a>(&mut self, body: &WhileBody<'a>) {
        match body {
            WhileBody::Statement(stmt) => self.analyze_statement(stmt),
            WhileBody::ColonDelimited(block) => {
                for inner in block.statements.iter() {
                    self.analyze_statement(inner);
                }
            }
        }
    }

    fn check_expression<'a>(&mut self, expr: &Expression<'a>) {
        match expr {
            // Check function calls
            Expression::Call(Call::Function(func_call)) => {
                self.check_function_call(func_call);
            }
            // Check method calls
            Expression::Call(Call::Method(method_call)) => {
                self.check_method_call(method_call);
            }
            // Recurse into expressions
            Expression::Binary(binary) => {
                self.check_expression(&binary.lhs);
                self.check_expression(&binary.rhs);
            }
            Expression::Parenthesized(p) => self.check_expression(&p.expression),
            Expression::UnaryPrefix(p) => self.check_expression(&p.operand),
            Expression::Conditional(c) => {
                self.check_expression(&c.condition);
                if let Some(then) = &c.then {
                    self.check_expression(then);
                }
                self.check_expression(&c.r#else);
            }
            Expression::Assignment(a) => {
                self.check_expression(&a.rhs);
            }
            _ => {}
        }
    }

    fn check_function_call<'a>(&mut self, func_call: &FunctionCall<'a>) {
        // Get the function name from the span (works for simple and qualified names)
        let func_span = func_call.function.span();
        let func_name = self.get_span_text(&func_span);

        // Skip dynamic calls (variable function calls)
        if func_name.starts_with('$') {
            return;
        }

        // Skip namespaced calls for now (we can't resolve them without autoloader)
        if func_name.contains('\\') {
            return;
        }

        let func_lower = func_name.to_lowercase();

        // Skip built-in functions (would need comprehensive type info)
        if self
            .builtin_functions
            .iter()
            .any(|f| f.eq_ignore_ascii_case(func_name))
        {
            return;
        }

        // Get function info
        if let Some(func_info) = self.functions.get(&func_lower).cloned() {
            self.check_arguments(&func_info, &func_call.argument_list, func_call.span());
        }
    }

    fn check_method_call<'a>(&mut self, method_call: &MethodCall<'a>) {
        // Get class name from $this or variable
        let class_name = if let Expression::Variable(Variable::Direct(var)) = &*method_call.object {
            let var_name = self.get_span_text(&var.span);
            if var_name == "$this" {
                self.current_class.clone()
            } else {
                // Try to get class from variable type
                self.variable_types.get(var_name).cloned()
            }
        } else {
            None
        };

        if let Some(class) = class_name {
            if let ClassLikeMemberSelector::Identifier(ident) = &method_call.method {
                let method_name = self.get_span_text(&ident.span);
                let full_name = format!("{}::{}", class, method_name).to_lowercase();

                if let Some(method_info) = self.methods.get(&full_name).cloned() {
                    self.check_arguments(&method_info, &method_call.argument_list, method_call.span());
                }
            }
        }
    }

    fn check_arguments<'a>(
        &mut self,
        func_info: &FunctionInfo,
        arg_list: &ArgumentList<'a>,
        _call_span: mago_span::Span,
    ) {
        for (i, arg) in arg_list.arguments.iter().enumerate() {
            if let Some(param) = func_info.params.get(i) {
                if let Some(expected_type) = &param.type_hint {
                    if let Some(actual_type) = self.infer_expression_type(arg.value()) {
                        if !self.types_compatible(expected_type, &actual_type, param.is_nullable) {
                            let (line, col) = self.get_line_col(arg.span().start.offset as usize);
                            self.issues.push(
                                Issue::error(
                                    "argument.type",
                                    format!(
                                        "Parameter #{} {} of {} expects {}, {} given.",
                                        i + 1,
                                        param.name,
                                        func_info.name,
                                        expected_type,
                                        actual_type
                                    ),
                                    self.file_path.clone(),
                                    line,
                                    col,
                                )
                                .with_identifier("argument.type"),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Infer the type of an expression
    fn infer_expression_type<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        match expr {
            Expression::Literal(literal) => match literal {
                Literal::String(_) => Some("string".to_string()),
                Literal::Integer(_) => Some("int".to_string()),
                Literal::Float(_) => Some("float".to_string()),
                Literal::True(_) | Literal::False(_) => Some("bool".to_string()),
                Literal::Null(_) => Some("null".to_string()),
            },
            Expression::Variable(Variable::Direct(var)) => {
                let var_name = self.get_span_text(&var.span);
                // Check parameter types first
                if let Some(type_name) = self.param_types.get(var_name) {
                    return Some(type_name.clone());
                }
                // Check variable types
                self.variable_types.get(var_name).cloned()
            }
            Expression::Array(_) => Some("array".to_string()),
            Expression::Instantiation(inst) => {
                // Extract class name from instantiation (new ClassName())
                if let Expression::Identifier(ident) = &*inst.class {
                    let class_name = self.get_span_text(&ident.span());
                    Some(class_name.to_string())
                } else {
                    Some("object".to_string())
                }
            }
            Expression::Closure(_) => Some("callable".to_string()),
            Expression::ArrowFunction(_) => Some("callable".to_string()),
            _ => None,
        }
    }

    /// Check if actual type is compatible with expected type
    fn types_compatible(&self, expected: &str, actual: &str, is_nullable: bool) -> bool {
        let expected_lower = expected.to_lowercase();
        let actual_lower = actual.to_lowercase();

        // Exact match
        if expected_lower == actual_lower {
            return true;
        }

        // Nullable check
        if is_nullable && actual_lower == "null" {
            return true;
        }

        // Mixed accepts anything
        if expected_lower == "mixed" {
            return true;
        }

        // At level 5, mixed can be passed to anything (level 9 will enforce the restriction)
        if actual_lower == "mixed" {
            return true;
        }

        // object accepts any object
        if expected_lower == "object" && actual_lower == "object" {
            return true;
        }

        // int|float compatibility (numeric)
        if (expected_lower == "float" || expected_lower == "double") && actual_lower == "int" {
            return true;
        }

        // callable accepts closures
        if expected_lower == "callable"
            && (actual_lower == "callable" || actual_lower == "closure")
        {
            return true;
        }

        // iterable accepts array
        if expected_lower == "iterable" && actual_lower == "array" {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_type_check_level() {
        let check = ArgumentTypeCheck;
        assert_eq!(check.level(), 5);
    }
}
