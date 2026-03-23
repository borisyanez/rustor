//! Argument type validation (Level 5)
//!
//! Checks that arguments passed to functions/methods match the expected types.

use crate::checks::{Check, CheckContext};
use crate::issue::Issue;
use crate::symbols::SymbolTable;
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
            current_namespace: String::new(),
            builtin_functions: ctx.builtin_functions,
            symbol_table: ctx.symbol_table,
            file_scope: ctx.scope,
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
    /// Current class context (FQN)
    current_class: Option<String>,
    /// Current namespace
    current_namespace: String,
    /// Built-in functions
    builtin_functions: &'s [&'static str],
    /// Symbol table for class hierarchy lookups
    symbol_table: Option<&'s SymbolTable>,
    /// File-level scope with namespace and use imports
    file_scope: Option<&'s crate::scope::Scope>,
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
            let (type_hint, mut is_nullable) = if let Some(hint) = &param.hint {
                (self.extract_type_name(hint), self.is_nullable_hint(hint))
            } else {
                (None, false)
            };

            // PHP 7 style: `string $x = null` — default null makes parameter implicitly nullable
            if !is_nullable {
                if let Some(default) = &param.default_value {
                    if let Expression::Literal(Literal::Null(_)) = &default.value {
                        is_nullable = true;
                    }
                }
            }

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
            // Union types - collect all members as pipe-separated string
            Hint::Union(union) => {
                let left = self.extract_type_name(union.left);
                let right = self.extract_type_name(union.right);
                match (left, right) {
                    (Some(l), Some(r)) => Some(format!("{}|{}", l, r)),
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    (None, None) => None,
                }
            }
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
                self.variable_types.clear();
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
                let fqn = if self.current_namespace.is_empty() {
                    class_name
                } else {
                    format!("{}\\{}", self.current_namespace, class_name)
                };
                self.current_class = Some(fqn);

                for member in class.members.iter() {
                    if let ClassLikeMember::Method(method) = member {
                        if let MethodBody::Concrete(body) = &method.body {
                            // Track parameter types
                            self.param_types.clear();
                            self.variable_types.clear();
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
                        } else {
                            // Unknown type - remove stale type to avoid false positives
                            self.variable_types.remove(&var_name);
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
            Statement::Namespace(ns) => {
                let prev_namespace = self.current_namespace.clone();
                if let Some(ref name) = ns.name {
                    let span = name.span();
                    self.current_namespace = self.get_span_text(&span).to_string();
                }
                match &ns.body {
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
                }
                self.current_namespace = prev_namespace;
            }
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

        // Get function info - first try local file definitions
        if let Some(func_info) = self.functions.get(&func_lower).cloned() {
            self.check_arguments(&func_info, &func_call.argument_list, func_call.span());
        }
        // Try symbol table for cross-file function signatures
        else if let Some(symbol_table) = self.symbol_table {
            if let Some(func_info_st) = symbol_table.get_function(func_name) {
                let params = func_info_st.parameters.iter().map(|p| {
                    let type_hint = p.type_.as_ref().map(|t| self.type_to_string(t));
                    let is_nullable = p.type_.as_ref().map_or(false, |t| {
                        matches!(t, crate::types::php_type::Type::Nullable(_))
                            || matches!(t, crate::types::php_type::Type::Null)
                    });
                    ParamInfo {
                        name: format!("${}", p.name),
                        type_hint,
                        is_nullable,
                        has_default: p.is_optional,
                    }
                }).collect();
                let fi = FunctionInfo { name: func_name.to_string(), params };
                self.check_arguments(&fi, &func_call.argument_list, func_call.span());
            }
        }
    }

    fn check_method_call<'a>(&mut self, method_call: &MethodCall<'a>) {
        // Get class name from $this or variable
        let class_name = if let Expression::Variable(Variable::Direct(var)) = &*method_call.object {
            let var_name = self.get_span_text(&var.span);
            if var_name == "$this" {
                self.current_class.clone()
            } else {
                // Try to get class from variable type or param type
                self.param_types.get(var_name).cloned()
                    .or_else(|| self.variable_types.get(var_name).cloned())
            }
        } else {
            None
        };

        if let Some(class) = class_name {
            if let ClassLikeMemberSelector::Identifier(ident) = &method_call.method {
                let method_name = self.get_span_text(&ident.span);
                let full_name = format!("{}::{}", class, method_name).to_lowercase();
                // Also try short class name for local file lookup
                let short_name = class.rsplit('\\').next().unwrap_or(&class);
                let short_full_name = format!("{}::{}", short_name, method_name).to_lowercase();

                // First try local file definitions (try FQN then short name)
                if let Some(method_info) = self.methods.get(&full_name).or_else(|| self.methods.get(&short_full_name)).cloned() {
                    self.check_arguments(&method_info, &method_call.argument_list, method_call.span());
                    return;
                }

                // Then try symbol table
                if let Some(symbol_table) = self.symbol_table {
                    let fqn = self.resolve_class_name_for_arg_check(&class);
                    if let Some(class_info) = symbol_table.get_class(&fqn).or_else(|| symbol_table.get_class(&class)) {
                        if let Some(method_info) = class_info.get_method(method_name) {
                            let func_info = self.method_info_to_function_info(&class, method_info);
                            self.check_arguments(&func_info, &method_call.argument_list, method_call.span());
                        }
                    }
                }
            }
        }
    }

    /// Convert a ClassMethodInfo from the symbol table to a FunctionInfo
    fn method_info_to_function_info(&self, class_name: &str, method: &crate::symbols::class_info::ClassMethodInfo) -> FunctionInfo {
        let full_name = format!("{}::{}", class_name, method.name);
        let params = method.parameters.iter().map(|p| {
            let type_hint = p.type_.as_ref().map(|t| self.type_to_string(t));
            let is_nullable = p.type_.as_ref().map_or(false, |t| {
                matches!(t, crate::types::php_type::Type::Nullable(_))
                    || matches!(t, crate::types::php_type::Type::Null)
            });
            ParamInfo {
                name: format!("${}", p.name),
                type_hint,
                is_nullable,
                has_default: p.is_optional,
            }
        }).collect();
        FunctionInfo { name: full_name, params }
    }

    /// Use ExpressionResolver for complex expression type inference
    fn resolve_expression_type<'a>(&self, expr: &Expression<'a>) -> Option<String> {
        if let Some(symbol_table) = self.symbol_table {
            let resolver = crate::resolver::expression_resolver::ExpressionResolver::new(symbol_table, self.source);
            // Start from file scope (has namespace + use imports) if available
            let mut scope = self.file_scope.cloned().unwrap_or_else(crate::scope::Scope::new);
            for (var_name, type_name) in &self.param_types {
                let var = var_name.trim_start_matches('$');
                if let Some(ty) = crate::types::phpdoc::parse_type_string(type_name) {
                    scope.set_variable(var, ty);
                }
            }
            for (var_name, type_name) in &self.variable_types {
                let var = var_name.trim_start_matches('$');
                if let Some(ty) = crate::types::phpdoc::parse_type_string(type_name) {
                    scope.set_variable(var, ty);
                }
            }
            let resolved = resolver.resolve(expr, &scope);
            let type_str = self.type_to_string(&resolved);
            if type_str != "mixed" {
                Some(type_str)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the return type of a builtin PHP function
    fn get_builtin_return_type(&self, func_name: &str) -> Option<String> {
        match func_name {
            // Functions that return string|false
            "file_get_contents" | "fgets" | "fread" | "readline"
            | "file" | "stream_get_contents" | "ob_get_contents"
            | "date" | "gmdate" | "strftime" | "iconv"
            | "mb_convert_encoding" | "mb_detect_encoding"
            | "base64_decode" | "hex2bin" | "quoted_printable_decode"
            | "json_encode" | "serialize" | "gzcompress" | "gzdeflate"
            | "gzencode" | "gzuncompress" | "gzinflate" => Some("string|false".to_string()),

            // Functions that return int|false
            "strpos" | "strrpos" | "strripos" | "stripos"
            | "ftell" | "fwrite" | "fputs"
            | "mktime" | "gmmktime" | "strtotime"
            | "filesize" | "fileatime" | "filectime" | "filemtime"
            | "preg_match" | "preg_match_all" => Some("int|false".to_string()),

            // Functions that return string
            "strtolower" | "strtoupper" | "trim" | "ltrim" | "rtrim"
            | "str_replace" | "str_repeat" | "str_pad"
            | "substr_replace" | "number_format"
            | "ucfirst" | "lcfirst" | "ucwords"
            | "nl2br" | "wordwrap" | "chunk_split"
            | "htmlspecialchars" | "htmlentities"
            | "htmlspecialchars_decode" | "html_entity_decode"
            | "urlencode" | "urldecode" | "rawurlencode" | "rawurldecode"
            | "base64_encode" | "md5" | "sha1" | "crc32"
            | "sprintf" | "vsprintf" | "implode" | "join"
            | "chr" | "str_word_count" | "money_format"
            | "bin2hex" | "quoted_printable_encode"
            | "mb_strtolower" | "mb_strtoupper" | "mb_substr" => Some("string".to_string()),

            // Functions that return int
            "strlen" | "count" | "sizeof" | "intval" | "ord"
            | "mb_strlen" | "mb_strpos" | "abs"
            | "ceil" | "floor" | "round"
            | "rand" | "mt_rand" | "random_int"
            | "time" | "intdiv" => Some("int".to_string()),

            // Functions that return float
            "floatval" | "doubleval" | "microtime" => Some("float".to_string()),

            // Functions that return bool
            "is_string" | "is_int" | "is_integer" | "is_float" | "is_double"
            | "is_bool" | "is_null" | "is_array" | "is_object" | "is_numeric"
            | "is_callable" | "is_resource" | "is_finite" | "is_nan" | "is_infinite"
            | "isset" | "empty" | "in_array" | "array_key_exists"
            | "file_exists" | "is_file" | "is_dir" | "is_readable" | "is_writable"
            | "class_exists" | "function_exists" | "method_exists" | "property_exists"
            | "defined" | "ctype_alpha" | "ctype_digit" | "ctype_alnum"
            | "filter_var" | "preg_quote" => Some("bool".to_string()),

            // Functions that return array
            "array_merge" | "array_combine" | "array_unique" | "array_reverse"
            | "array_flip" | "array_keys" | "array_values" | "array_chunk"
            | "array_slice" | "array_splice" | "array_diff" | "array_intersect"
            | "array_map" | "array_filter" | "compact" | "range"
            | "explode" | "str_split" | "preg_split" | "glob"
            | "scandir" | "get_object_vars" | "get_class_methods"
            | "func_get_args" => Some("array".to_string()),

            // Functions that return array|false
            "parse_url" | "getimagesize" | "stat" | "lstat"
            | "pathinfo" => Some("array|false".to_string()),

            // Functions that return mixed
            "json_decode" | "unserialize" | "array_pop" | "array_shift"
            | "current" | "next" | "prev" | "end" | "reset" => None, // mixed = no useful type

            _ => None,
        }
    }

    /// Convert a Type to a string representation for comparison
    fn type_to_string(&self, ty: &crate::types::php_type::Type) -> String {
        use crate::types::php_type::Type;
        match ty {
            Type::Int | Type::ConstantInt(_) | Type::IntRange { .. } => "int".to_string(),
            Type::String | Type::ConstantString(_) | Type::NonEmptyString | Type::NumericString => "string".to_string(),
            Type::Float | Type::ConstantFloat(_) => "float".to_string(),
            Type::Bool | Type::ConstantBool(_) => "bool".to_string(),
            Type::Null => "null".to_string(),
            Type::Void => "void".to_string(),
            Type::Never => "never".to_string(),
            Type::Mixed => "mixed".to_string(),
            Type::Object { class_name: Some(name) } => name.clone(),
            Type::Object { class_name: None } => "object".to_string(),
            Type::GenericObject { class_name, .. } => class_name.clone(),
            Type::Nullable(inner) => {
                let inner_str = self.type_to_string(inner);
                format!("?{}", inner_str)
            }
            Type::Union(types) => {
                types.iter().map(|t| self.type_to_string(t)).collect::<Vec<_>>().join("|")
            }
            Type::Intersection(types) => {
                types.iter().map(|t| self.type_to_string(t)).collect::<Vec<_>>().join("&")
            }
            Type::Array { .. } | Type::NonEmptyArray { .. } => "array".to_string(),
            Type::List { .. } => "array".to_string(),
            Type::Iterable { .. } => "iterable".to_string(),
            Type::Callable | Type::Closure => "callable".to_string(),
            Type::SelfType => "self".to_string(),
            Type::Static => "static".to_string(),
            Type::Parent => "parent".to_string(),
            _ => "mixed".to_string(),
        }
    }

    /// Resolve a class name to its fully qualified name using use imports
    fn resolve_class_name_for_arg_check(&self, name: &str) -> String {
        // For now, just return as-is — we rely on symbol_table's case-insensitive lookup
        // and the fact that get_class already handles FQN
        name.to_string()
    }

    fn check_arguments<'a>(
        &mut self,
        func_info: &FunctionInfo,
        arg_list: &ArgumentList<'a>,
        _call_span: mago_span::Span,
    ) {
        // Skip calls with named arguments — positional index matching is invalid there
        if arg_list.arguments.iter().any(|a| matches!(a, Argument::Named(_))) {
            return;
        }

        for (i, arg) in arg_list.arguments.iter().enumerate() {
            if let Some(param) = func_info.params.get(i) {
                if let Some(expected_type) = &param.type_hint {
                    if let Some(actual_type) = self.infer_expression_type(arg.value()) {
                        if !self.types_compatible(expected_type, &actual_type, param.is_nullable) {
                            let (line, col) = self.get_line_col(arg.span().start.offset as usize);
                            // Format name with "method" or "function" prefix and () suffix
                            let display_name = if func_info.name.contains("::") {
                                format!("method {}()", func_info.name)
                            } else {
                                format!("function {}", func_info.name)
                            };
                            self.issues.push(
                                Issue::error(
                                    "argument.type",
                                    format!(
                                        "Parameter #{} {} of {} expects {}, {} given.",
                                        i + 1,
                                        param.name,
                                        display_name,
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
            Expression::Array(_) | Expression::LegacyArray(_) => Some("array".to_string()),
            Expression::Instantiation(inst) => {
                if let Expression::Identifier(ident) = &*inst.class {
                    let class_name = self.get_span_text(&ident.span());
                    Some(class_name.to_string())
                } else {
                    Some("object".to_string())
                }
            }
            Expression::Closure(_) | Expression::ArrowFunction(_) => Some("callable".to_string()),
            // Use ExpressionResolver for function/method calls
            Expression::Call(_) => self.resolve_expression_type(expr),
            _ => None,
        }
    }

    /// Check if actual type is compatible with expected type
    fn types_compatible(&self, expected: &str, actual: &str, is_nullable: bool) -> bool {
        // Handle union types in expected (pipe-separated, e.g. "string|array")
        if expected.contains('|') {
            let exp_members: Vec<&str> = expected.split('|').map(|s| s.trim()).collect();
            // If actual is also a union, each actual member must fit at least one expected member
            if actual.contains('|') {
                return actual.split('|').map(|s| s.trim()).all(|act| {
                    exp_members.iter().any(|exp| self.types_compatible(exp, act, is_nullable))
                });
            }
            return exp_members.iter().any(|member| self.types_compatible(member, actual, is_nullable));
        }

        // Handle union types in actual only (e.g. actual is "string|false")
        // At level 5-6, PHPStan accepts if ANY member of the actual union is compatible
        // (stricter checking happens at level 8+)
        if actual.contains('|') {
            return actual.split('|').map(|s| s.trim()).any(|member| {
                self.types_compatible(expected, member, is_nullable)
            });
        }

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

        // object accepts any class instance (any non-primitive type)
        if expected_lower == "object" && !Self::is_primitive(actual) {
            return true;
        }
        // object accepts literal "object" type
        if expected_lower == "object" && actual_lower == "object" {
            return true;
        }

        // At level 5-6, accept nullable actual for non-nullable expected
        // PHPStan uses type narrowing to know the value is non-null at call site
        if actual_lower.starts_with('?') && expected_lower == &actual_lower[1..] {
            return true;
        }
        // Also handle "Type|null" format
        if actual_lower.ends_with("|null") && expected_lower == &actual_lower[..actual_lower.len()-5] {
            return true;
        }
        if actual_lower.starts_with("null|") && expected_lower == &actual_lower[5..] {
            return true;
        }

        // bool accepts true/false (constant bool subtypes)
        if expected_lower == "bool" && (actual_lower == "true" || actual_lower == "false") {
            return true;
        }

        // true/false are subtypes of bool
        if (expected_lower == "true" || expected_lower == "false") && actual_lower == "bool" {
            // bool is wider than true/false, so this is actually NOT compatible
            // but at level 5-6, PHPStan doesn't report this
            return true;
        }

        // Handle nullable format: ?string = string|null
        if expected_lower.starts_with('?') {
            let inner = &expected_lower[1..];
            if actual_lower == inner || actual_lower == "null" {
                return true;
            }
        }
        if actual_lower.starts_with('?') {
            let inner = &actual_lower[1..];
            if expected_lower == inner {
                return true; // ?string passed to string param (the non-null case)
            }
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

        // Stringable accepts string
        if expected_lower == "stringable" && actual_lower == "string" {
            return true;
        }

        // string|Stringable accepts string
        if expected_lower == "string" && actual_lower == "stringable" {
            return true;
        }

        // If expected is a class/interface name, check class hierarchy
        if !Self::is_primitive(expected) {
            if let Some(st) = self.symbol_table {
                // Check if actual is a subtype of expected via inheritance/interfaces
                if self.is_subtype(st, actual, expected, 0) {
                    return true;
                }
            } else {
                // No symbol table - can't verify class hierarchy, be conservative
                return true;
            }
        }

        // Fallback: try Type-based comparison with hierarchy for class types
        if let Some(st) = self.symbol_table {
            if !Self::is_primitive(expected) && !Self::is_primitive(actual) {
                use crate::types::php_type::Type;
                let exp_type = Type::object(expected.to_string());
                let act_type = Type::object(actual.to_string());
                if exp_type.accepts_with_hierarchy(&act_type, false, st).yes() {
                    return true;
                }
            }
        }

        false
    }

    /// Check if `actual` is a subtype of `expected` (via inheritance or interface implementation)
    fn is_subtype(&self, st: &SymbolTable, actual: &str, expected: &str, depth: u8) -> bool {
        if depth > 20 {
            return false;
        }
        let actual_norm = actual.trim_start_matches('\\');
        let expected_norm = expected.trim_start_matches('\\');

        if actual_norm.eq_ignore_ascii_case(expected_norm) {
            return true;
        }

        let actual_lower = actual_norm.to_lowercase();
        if let Some(class_info) = st.get_class(&actual_lower) {
            // Check interfaces
            for iface in &class_info.interfaces {
                if iface.trim_start_matches('\\').eq_ignore_ascii_case(expected_norm) {
                    return true;
                }
                if self.is_subtype(st, iface, expected_norm, depth + 1) {
                    return true;
                }
            }
            // Check parent class
            if let Some(ref parent) = class_info.parent {
                if self.is_subtype(st, parent, expected_norm, depth + 1) {
                    return true;
                }
            }
        } else {
            // Class not found in symbol table - be conservative, assume compatible
            return true;
        }

        false
    }

    /// Returns true if the type name is a PHP primitive (not a class/interface name)
    fn is_primitive(type_name: &str) -> bool {
        matches!(
            type_name.to_lowercase().as_str(),
            "int" | "integer" | "float" | "double" | "string" | "bool" | "boolean"
                | "array" | "object" | "null" | "void" | "never" | "resource"
                | "callable" | "iterable" | "mixed" | "true" | "false"
        )
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
