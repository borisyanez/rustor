//! Expression type resolver
//!
//! Resolves the type of PHP expressions based on scope and symbol table.

use crate::scope::Scope;
use crate::symbols::SymbolTable;
use crate::types::Type;
use crate::types::phpdoc::parse_type_string;
use mago_span::HasSpan;
use mago_syntax::ast::*;

/// Resolves expression types
pub struct ExpressionResolver<'a> {
    symbol_table: &'a SymbolTable,
    source: &'a str,
}

impl<'a> ExpressionResolver<'a> {
    /// Create a new expression resolver
    pub fn new(symbol_table: &'a SymbolTable, source: &'a str) -> Self {
        Self { symbol_table, source }
    }

    /// Resolve the type of an expression
    pub fn resolve(&self, expr: &Expression<'_>, scope: &Scope) -> Type {
        match expr {
            Expression::Literal(lit) => self.resolve_literal(lit),
            Expression::Variable(var) => self.resolve_variable(var, scope),
            Expression::Array(arr) => self.resolve_array(arr, scope),
            Expression::LegacyArray(arr) => self.resolve_legacy_array(arr, scope),
            Expression::List(_) => Type::mixed_array(),
            Expression::ArrayAccess(access) => self.resolve_array_access(access, scope),
            Expression::ArrayAppend(_) => Type::Mixed,
            Expression::Parenthesized(paren) => self.resolve(&paren.expression, scope),
            Expression::Closure(closure) => self.resolve_closure(closure),
            Expression::ArrowFunction(_) => Type::Closure,
            Expression::Instantiation(inst) => self.resolve_instantiation(inst, scope),
            Expression::Clone(clone) => self.resolve(&clone.object, scope),
            Expression::Access(access) => self.resolve_access(access, scope),
            Expression::Call(call) => self.resolve_call(call, scope),
            Expression::Conditional(cond) => self.resolve_conditional(cond, scope),
            Expression::Match(m) => self.resolve_match(m, scope),
            Expression::Yield(_) => Type::Mixed,
            Expression::Throw(_) => Type::Never,
            Expression::Binary(binary) => self.resolve_binary(binary, scope),
            Expression::UnaryPrefix(unary) => self.resolve_unary_prefix(unary, scope),
            Expression::UnaryPostfix(unary) => self.resolve_unary_postfix(unary, scope),
            Expression::Assignment(assign) => self.resolve(&assign.rhs, scope),
            Expression::MagicConstant(mc) => self.resolve_magic_constant(mc),
            Expression::Identifier(_) => Type::Mixed,
            Expression::Static(_) => Type::Static,
            Expression::Self_(_) => Type::SelfType,
            Expression::Parent(_) => Type::Parent,
            Expression::Construct(construct) => self.resolve_construct(construct),
            _ => Type::Mixed,
        }
    }

    /// Resolve literal expression type
    fn resolve_literal(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Null(_) => Type::Null,
            Literal::False(_) => Type::ConstantBool(false),
            Literal::True(_) => Type::ConstantBool(true),
            Literal::Integer(i) => {
                let text = self.get_span_text(&i.span());
                if let Ok(val) = text.parse::<i64>() {
                    Type::ConstantInt(val)
                } else {
                    Type::Int
                }
            }
            Literal::Float(_) => Type::Float,
            Literal::String(s) => {
                let text = self.get_span_text(&s.span());
                // Remove quotes
                let content = text.trim_matches(|c| c == '"' || c == '\'');
                if content.len() < 100 {
                    Type::ConstantString(content.to_string())
                } else {
                    Type::String
                }
            }
            _ => Type::String, // CompositeString or other string literals
        }
    }

    /// Resolve variable type
    fn resolve_variable(&self, var: &Variable, scope: &Scope) -> Type {
        match var {
            Variable::Direct(direct) => {
                let name = self.get_span_text(&direct.span);
                let name = name.trim_start_matches('$');
                scope.get_variable_type(name).unwrap_or(Type::Mixed)
            }
            Variable::Indirect(_) => Type::Mixed,
            Variable::Nested(_) => Type::Mixed,
        }
    }

    /// Resolve array literal type
    fn resolve_array(&self, arr: &Array, scope: &Scope) -> Type {
        if arr.elements.is_empty() {
            return Type::Array {
                key: Box::new(Type::Mixed),
                value: Box::new(Type::Mixed),
            };
        }

        let mut key_types = Vec::new();
        let mut value_types = Vec::new();
        let mut is_list = true;
        let mut expected_index = 0i64;

        for element in arr.elements.iter() {
            match element {
                ArrayElement::KeyValue(kv) => {
                    key_types.push(self.resolve(&kv.key, scope));
                    value_types.push(self.resolve(&kv.value, scope));
                    is_list = false;
                }
                ArrayElement::Value(val) => {
                    key_types.push(Type::ConstantInt(expected_index));
                    value_types.push(self.resolve(&val.value, scope));
                    expected_index += 1;
                }
                ArrayElement::Variadic(_) => {
                    is_list = false;
                }
                ArrayElement::Missing(_) => {
                    expected_index += 1;
                }
            }
        }

        let key_type = if key_types.is_empty() {
            Type::Mixed
        } else if key_types.iter().all(|t| matches!(t, Type::ConstantInt(_))) && is_list {
            Type::Int
        } else {
            Type::Mixed
        };

        let value_type = if value_types.is_empty() {
            Type::Mixed
        } else if value_types.len() == 1 {
            value_types.into_iter().next().unwrap().generalize()
        } else {
            value_types.into_iter().fold(Type::Never, |acc, t| acc.union_with(t)).generalize()
        };

        if is_list {
            Type::List {
                value: Box::new(value_type),
            }
        } else {
            Type::Array {
                key: Box::new(key_type),
                value: Box::new(value_type),
            }
        }
    }

    /// Resolve legacy array (array(...)) type
    fn resolve_legacy_array(&self, arr: &LegacyArray, scope: &Scope) -> Type {
        if arr.elements.is_empty() {
            return Type::mixed_array();
        }

        // Similar logic to resolve_array
        Type::mixed_array()
    }

    /// Resolve array access type
    fn resolve_array_access(&self, access: &ArrayAccess, scope: &Scope) -> Type {
        let array_type = self.resolve(&access.array, scope);

        match array_type {
            Type::Array { value, .. } | Type::List { value } | Type::NonEmptyArray { value, .. } => {
                *value
            }
            Type::String | Type::ConstantString(_) | Type::NonEmptyString => Type::String,
            _ => Type::Mixed,
        }
    }

    /// Resolve closure type
    fn resolve_closure(&self, _closure: &Closure) -> Type {
        Type::Closure
    }

    /// Resolve instantiation (new) expression type
    fn resolve_instantiation(&self, inst: &Instantiation, scope: &Scope) -> Type {
        match &*inst.class {
            Expression::Identifier(ident) => {
                let class_name = self.get_span_text(&ident.span()).to_string();
                let resolved = scope.resolve_class_name(&class_name);
                Type::Object {
                    class_name: Some(resolved),
                }
            }
            Expression::Self_(_) => scope.get_this_type().unwrap_or(Type::SelfType),
            Expression::Static(_) => Type::Static,
            Expression::Parent(_) => Type::Parent,
            _ => Type::Object { class_name: None },
        }
    }

    /// Resolve access expressions (property fetch, static property, class constant)
    fn resolve_access(&self, access: &Access, scope: &Scope) -> Type {
        match access {
            Access::Property(prop_access) => {
                let object_type = self.resolve(&prop_access.object, scope);
                self.resolve_property_fetch(&object_type, &prop_access.property)
            }
            Access::NullSafeProperty(prop_access) => {
                let object_type = self.resolve(&prop_access.object, scope);
                let inner = self.resolve_property_fetch(&object_type, &prop_access.property);
                Type::Nullable(Box::new(inner))
            }
            Access::StaticProperty(sp) => {
                let class_name = self.resolve_class_expression(&sp.class, scope);
                if let Variable::Direct(direct) = &sp.property {
                    let prop_name = self.get_span_text(&direct.span).trim_start_matches('$');
                    if let Some(class) = self.symbol_table.get_class(&class_name) {
                        if let Some(prop) = class.get_property(prop_name) {
                            return prop.type_.clone().unwrap_or(Type::Mixed);
                        }
                    }
                }
                Type::Mixed
            }
            Access::ClassConstant(cc) => {
                let class_name = self.resolve_class_expression(&cc.class, scope);
                if let ClassLikeConstantSelector::Identifier(ident) = &cc.constant {
                    let const_name = &ident.value;
                    if *const_name == "class" {
                        return Type::ClassString {
                            class_name: Some(class_name),
                        };
                    }
                    if let Some(class) = self.symbol_table.get_class(&class_name) {
                        if let Some(constant) = class.get_constant(const_name) {
                            return constant.type_.clone().unwrap_or(Type::Mixed);
                        }
                    }
                }
                Type::Mixed
            }
        }
    }

    /// Resolve class expression to a class name string
    fn resolve_class_expression(&self, expr: &Expression, scope: &Scope) -> String {
        match expr {
            Expression::Identifier(ident) => {
                let name = self.get_span_text(&ident.span()).to_string();
                scope.resolve_class_name(&name)
            }
            Expression::Self_(_) => {
                scope.class_context().map_or("self".to_string(), |c| c.name.clone())
            }
            Expression::Static(_) => "static".to_string(),
            Expression::Parent(_) => "parent".to_string(),
            _ => String::new(),
        }
    }

    /// Resolve property fetch type
    fn resolve_property_fetch(&self, object_type: &Type, property: &ClassLikeMemberSelector) -> Type {
        if let Type::Object { class_name: Some(class_name) } = object_type {
            if let ClassLikeMemberSelector::Identifier(ident) = property {
                let prop_name = ident.value;
                if let Some(class) = self.symbol_table.get_class(class_name) {
                    if let Some(prop) = class.get_property(prop_name) {
                        return prop.type_.clone().unwrap_or(Type::Mixed);
                    }
                }
            }
        }
        Type::Mixed
    }

    /// Resolve method call type
    fn resolve_method_call(&self, object_type: &Type, method: &ClassLikeMemberSelector) -> Type {
        if let Type::Object { class_name: Some(class_name) } = object_type {
            if let ClassLikeMemberSelector::Identifier(ident) = method {
                let method_name = ident.value;
                // Search class hierarchy for the method
                if let Some(ret) = self.find_method_return_in_hierarchy(class_name, method_name) {
                    return ret;
                }
            }
        }
        Type::Mixed
    }

    /// Search class hierarchy for a method's return type
    fn find_method_return_in_hierarchy(&self, class_name: &str, method_name: &str) -> Option<Type> {
        if let Some(class) = self.symbol_table.get_class(class_name) {
            if let Some(method_info) = class.get_method(method_name) {
                return Some(method_info.return_type.clone().unwrap_or(Type::Mixed));
            }
            // Check parent
            if let Some(parent) = &class.parent {
                if let Some(ret) = self.find_method_return_in_hierarchy(parent, method_name) {
                    return Some(ret);
                }
            }
            // Check interfaces
            for iface in &class.interfaces {
                if let Some(ret) = self.find_method_return_in_hierarchy(iface, method_name) {
                    return Some(ret);
                }
            }
            // Check traits
            for trait_name in &class.traits {
                if let Some(ret) = self.find_method_return_in_hierarchy(trait_name, method_name) {
                    return Some(ret);
                }
            }
        }
        None
    }

    /// Resolve call expressions (function, method, static method, nullsafe method)
    fn resolve_call(&self, call: &Call, scope: &Scope) -> Type {
        match call {
            Call::Function(func_call) => {
                if let Expression::Identifier(ident) = &*func_call.function {
                    let func_name = self.get_span_text(&ident.span()).to_string();
                    if let Some(t) = self.resolve_builtin_function(&func_name) {
                        return t;
                    }
                    // Try direct name, then FQN via scope
                    let resolved = scope.resolve_class_name(&func_name);
                    if let Some(func) = self.symbol_table.get_function(&resolved)
                        .or_else(|| self.symbol_table.get_function(&func_name))
                    {
                        return func.return_type.clone().unwrap_or(Type::Mixed);
                    }
                }
                Type::Mixed
            }
            Call::Method(method_call) => {
                let object_type = self.resolve(&method_call.object, scope);
                self.resolve_method_call(&object_type, &method_call.method)
            }
            Call::NullSafeMethod(method_call) => {
                let object_type = self.resolve(&method_call.object, scope);
                let inner = self.resolve_method_call(&object_type, &method_call.method);
                Type::Nullable(Box::new(inner))
            }
            Call::StaticMethod(static_call) => {
                let class_name = self.resolve_class_expression(&static_call.class, scope);
                if let ClassLikeMemberSelector::Identifier(ident) = &static_call.method {
                    let method_name = ident.value;
                    if let Some(ret) = self.find_method_return_in_hierarchy(&class_name, method_name) {
                        return ret;
                    }
                }
                Type::Mixed
            }
            _ => Type::Mixed,
        }
    }

    /// Resolve built-in function return types
    fn resolve_builtin_function(&self, func_name: &str) -> Option<Type> {
        match func_name.to_lowercase().as_str() {
            "strlen" | "count" | "sizeof" => Some(Type::Int),
            "strval" | "trim" | "ltrim" | "rtrim" | "strtolower" | "strtoupper" => Some(Type::String),
            "intval" => Some(Type::Int),
            "floatval" | "doubleval" => Some(Type::Float),
            "boolval" => Some(Type::Bool),
            "array_keys" => Some(Type::list(Type::Mixed)),
            "array_values" => Some(Type::list(Type::Mixed)),
            "array_merge" | "array_replace" => Some(Type::mixed_array()),
            "array_filter" | "array_map" | "array_reverse" => Some(Type::mixed_array()),
            "array_pop" | "array_shift" => Some(Type::Mixed),
            "is_null" | "is_array" | "is_string" | "is_int" | "is_float" | "is_bool"
            | "is_object" | "is_callable" | "is_numeric" | "isset" | "empty" => Some(Type::Bool),
            "json_encode" => Some(Type::union(vec![Type::String, Type::ConstantBool(false)])),
            "json_decode" => Some(Type::Mixed),
            "file_get_contents" => Some(Type::union(vec![Type::String, Type::ConstantBool(false)])),
            "file_exists" | "is_file" | "is_dir" | "is_readable" | "is_writable" => Some(Type::Bool),
            "class_exists" | "method_exists" | "property_exists" | "function_exists" => Some(Type::Bool),
            "get_class" => Some(Type::union(vec![Type::String, Type::ConstantBool(false)])),
            "gettype" => Some(Type::String),
            "time" | "strtotime" => Some(Type::Int),
            "microtime" => Some(Type::union(vec![Type::Float, Type::String])),
            "date" | "gmdate" => Some(Type::String),
            "sprintf" | "vsprintf" => Some(Type::String),
            "preg_match" | "preg_match_all" => Some(Type::union(vec![Type::Int, Type::ConstantBool(false)])),
            "preg_replace" => Some(Type::union(vec![Type::String, Type::mixed_array(), Type::Null])),
            "substr" => Some(Type::union(vec![Type::String, Type::ConstantBool(false)])),
            "strstr" | "stristr" => Some(Type::union(vec![Type::String, Type::ConstantBool(false)])),
            "strpos" | "strrpos" | "stripos" | "strripos" => Some(Type::union(vec![Type::Int, Type::ConstantBool(false)])),
            "str_replace" | "str_ireplace" => Some(Type::union(vec![Type::String, Type::mixed_array()])),
            "implode" | "join" => Some(Type::String),
            "explode" => Some(Type::list(Type::String)),
            "array_unique" | "array_flip" | "array_combine" => Some(Type::mixed_array()),
            "array_slice" | "array_splice" | "array_chunk" | "array_diff" | "array_intersect" => Some(Type::mixed_array()),
            "compact" | "range" => Some(Type::mixed_array()),
            "str_split" | "preg_split" => Some(Type::union(vec![Type::list(Type::String), Type::ConstantBool(false)])),
            "glob" => Some(Type::union(vec![Type::list(Type::String), Type::ConstantBool(false)])),
            "abs" => Some(Type::union(vec![Type::Int, Type::Float])),
            "ceil" | "floor" | "round" => Some(Type::union(vec![Type::Int, Type::Float])),
            "rand" | "mt_rand" | "random_int" => Some(Type::Int),
            "array_key_exists" | "in_array" => Some(Type::Bool),
            "array_search" => Some(Type::union(vec![Type::Mixed, Type::ConstantBool(false)])),
            "str_pad" | "str_repeat" | "str_word_count" => Some(Type::String),
            "ucfirst" | "lcfirst" | "ucwords" | "wordwrap" | "nl2br" => Some(Type::String),
            "htmlspecialchars" | "htmlentities" | "htmlspecialchars_decode" | "html_entity_decode" => Some(Type::String),
            "urlencode" | "urldecode" | "rawurlencode" | "rawurldecode" => Some(Type::String),
            "base64_encode" | "md5" | "sha1" | "crc32" | "bin2hex" | "hex2bin" => Some(Type::String),
            "number_format" => Some(Type::String),
            "chr" | "ord" => Some(Type::union(vec![Type::String, Type::Int])),
            "min" | "max" => Some(Type::Mixed),
            "array_sum" | "array_product" => Some(Type::union(vec![Type::Int, Type::Float])),
            "parse_url" => Some(Type::union(vec![Type::mixed_array(), Type::ConstantBool(false)])),
            "pathinfo" => Some(Type::union(vec![Type::mixed_array(), Type::String])),
            "realpath" | "dirname" | "basename" => Some(Type::union(vec![Type::String, Type::ConstantBool(false)])),
            "fopen" => Some(Type::union(vec![Type::Object { class_name: None }, Type::ConstantBool(false)])),
            "fgets" | "fread" => Some(Type::union(vec![Type::String, Type::ConstantBool(false)])),
            "fwrite" | "fputs" => Some(Type::union(vec![Type::Int, Type::ConstantBool(false)])),
            "fclose" | "feof" | "fflush" => Some(Type::Bool),
            "header" | "setcookie" => Some(Type::Void),
            "var_dump" | "print_r" | "var_export" => Some(Type::Mixed),
            "serialize" => Some(Type::String),
            "unserialize" => Some(Type::Mixed),
            _ => None,
        }
    }

    /// Resolve conditional (ternary) expression type
    fn resolve_conditional(&self, cond: &Conditional, scope: &Scope) -> Type {
        let if_true = cond.then.as_ref()
            .map(|e| self.resolve(e, scope))
            .unwrap_or_else(|| self.resolve(&cond.condition, scope));
        let if_false = self.resolve(&cond.r#else, scope);
        if_true.union_with(if_false)
    }

    /// Resolve match expression type
    fn resolve_match(&self, m: &Match, scope: &Scope) -> Type {
        let mut result_type = Type::Never;
        for arm in m.arms.iter() {
            match arm {
                MatchArm::Expression(expr_arm) => {
                    let arm_type = self.resolve(&expr_arm.expression, scope);
                    result_type = result_type.union_with(arm_type);
                }
                MatchArm::Default(default_arm) => {
                    let arm_type = self.resolve(&default_arm.expression, scope);
                    result_type = result_type.union_with(arm_type);
                }
            }
        }
        result_type
    }

    /// Resolve binary expression type
    fn resolve_binary(&self, binary: &Binary, scope: &Scope) -> Type {
        match &binary.operator {
            BinaryOperator::Addition(_) | BinaryOperator::Subtraction(_)
            | BinaryOperator::Multiplication(_) | BinaryOperator::Division(_)
            | BinaryOperator::Modulo(_) | BinaryOperator::Exponentiation(_) => {
                let left = self.resolve(&binary.lhs, scope);
                let right = self.resolve(&binary.rhs, scope);
                if matches!(left, Type::Int | Type::ConstantInt(_))
                    && matches!(right, Type::Int | Type::ConstantInt(_)) {
                    if matches!(binary.operator, BinaryOperator::Division(_)) {
                        Type::union(vec![Type::Int, Type::Float])
                    } else {
                        Type::Int
                    }
                } else {
                    Type::union(vec![Type::Int, Type::Float])
                }
            }
            BinaryOperator::StringConcat(_) => Type::String,
            BinaryOperator::BitwiseAnd(_) | BinaryOperator::BitwiseOr(_)
            | BinaryOperator::BitwiseXor(_) | BinaryOperator::LeftShift(_)
            | BinaryOperator::RightShift(_) => Type::Int,
            BinaryOperator::Equal(_) | BinaryOperator::Identical(_)
            | BinaryOperator::NotEqual(_) | BinaryOperator::NotIdentical(_)
            | BinaryOperator::LessThan(_) | BinaryOperator::LessThanOrEqual(_)
            | BinaryOperator::GreaterThan(_) | BinaryOperator::GreaterThanOrEqual(_)
            | BinaryOperator::Spaceship(_) => Type::Bool,
            BinaryOperator::And(_) | BinaryOperator::Or(_)
            | BinaryOperator::LowAnd(_) | BinaryOperator::LowOr(_)
            | BinaryOperator::LowXor(_) => Type::Bool,
            BinaryOperator::Instanceof(_) => Type::Bool,
            BinaryOperator::NullCoalesce(_) => {
                let left = self.resolve(&binary.lhs, scope).remove_null();
                let right = self.resolve(&binary.rhs, scope);
                left.union_with(right)
            }
            _ => Type::Mixed,
        }
    }

    /// Resolve unary prefix expression type
    fn resolve_unary_prefix(&self, unary: &UnaryPrefix, scope: &Scope) -> Type {
        match &unary.operator {
            UnaryPrefixOperator::Not(_) => Type::Bool,
            UnaryPrefixOperator::BitwiseNot(_) => Type::Int,
            UnaryPrefixOperator::Negation(_) | UnaryPrefixOperator::Plus(_) => {
                let inner = self.resolve(&unary.operand, scope);
                match inner {
                    Type::Int | Type::ConstantInt(_) => Type::Int,
                    Type::Float | Type::ConstantFloat(_) => Type::Float,
                    _ => Type::union(vec![Type::Int, Type::Float]),
                }
            }
            UnaryPrefixOperator::PreIncrement(_) | UnaryPrefixOperator::PreDecrement(_) => {
                let inner = self.resolve(&unary.operand, scope);
                match inner {
                    Type::Int | Type::ConstantInt(_) => Type::Int,
                    Type::Float | Type::ConstantFloat(_) => Type::Float,
                    _ => Type::Mixed,
                }
            }
            UnaryPrefixOperator::Reference(_) => self.resolve(&unary.operand, scope),
            UnaryPrefixOperator::ErrorControl(_) => self.resolve(&unary.operand, scope),
            UnaryPrefixOperator::IntCast(_, _) | UnaryPrefixOperator::IntegerCast(_, _) => Type::Int,
            UnaryPrefixOperator::BoolCast(_, _) | UnaryPrefixOperator::BooleanCast(_, _) => Type::Bool,
            UnaryPrefixOperator::FloatCast(_, _) | UnaryPrefixOperator::DoubleCast(_, _)
            | UnaryPrefixOperator::RealCast(_, _) => Type::Float,
            UnaryPrefixOperator::StringCast(_, _) | UnaryPrefixOperator::BinaryCast(_, _) => Type::String,
            UnaryPrefixOperator::ArrayCast(_, _) => Type::mixed_array(),
            UnaryPrefixOperator::ObjectCast(_, _) => Type::Object { class_name: None },
            UnaryPrefixOperator::UnsetCast(_, _) => Type::Null,
            UnaryPrefixOperator::VoidCast(_, _) => Type::Void,
        }
    }

    /// Resolve unary postfix expression type
    fn resolve_unary_postfix(&self, unary: &UnaryPostfix, scope: &Scope) -> Type {
        let inner = self.resolve(&unary.operand, scope);
        match inner {
            Type::Int | Type::ConstantInt(_) => Type::Int,
            Type::Float | Type::ConstantFloat(_) => Type::Float,
            _ => Type::Mixed,
        }
    }

    /// Resolve construct expressions (isset, empty, eval, include, etc.)
    fn resolve_construct(&self, construct: &Construct) -> Type {
        match construct {
            Construct::Isset(_) | Construct::Empty(_) | Construct::Print(_) => Type::Bool,
            Construct::Eval(_) => Type::Mixed,
            Construct::Include(_) | Construct::IncludeOnce(_)
            | Construct::Require(_) | Construct::RequireOnce(_) => Type::Mixed,
            Construct::Exit(_) | Construct::Die(_) => Type::Never,
        }
    }

    /// Resolve magic constant type
    fn resolve_magic_constant(&self, mc: &MagicConstant) -> Type {
        match mc {
            MagicConstant::Line(_) => Type::Int,
            MagicConstant::File(_) | MagicConstant::Directory(_)
            | MagicConstant::Function(_) | MagicConstant::Class(_)
            | MagicConstant::Method(_) | MagicConstant::Namespace(_)
            | MagicConstant::Trait(_) | MagicConstant::Property(_) => Type::String,
        }
    }

    /// Parse a type hint node into a Type
    pub fn resolve_type_hint(&self, hint: &Hint, scope: &Scope) -> Type {
        match hint {
            Hint::Void(_) => Type::Void,
            Hint::Never(_) => Type::Never,
            Hint::Null(_) => Type::Null,
            Hint::True(_) => Type::ConstantBool(true),
            Hint::False(_) => Type::ConstantBool(false),
            Hint::Bool(_) => Type::Bool,
            Hint::Integer(_) => Type::Int,
            Hint::Float(_) => Type::Float,
            Hint::String(_) => Type::String,
            Hint::Array(_) => Type::mixed_array(),
            Hint::Object(_) => Type::Object { class_name: None },
            Hint::Mixed(_) => Type::Mixed,
            Hint::Callable(_) => Type::Callable,
            Hint::Iterable(_) => Type::Iterable {
                key: Box::new(Type::Mixed),
                value: Box::new(Type::Mixed),
            },
            Hint::Static(_) => Type::Static,
            Hint::Self_(_) => Type::SelfType,
            Hint::Parent(_) => Type::Parent,
            Hint::Identifier(ident) => {
                let name = self.get_span_text(&ident.span());
                let resolved = scope.resolve_class_name(&name);
                Type::Object {
                    class_name: Some(resolved),
                }
            }
            Hint::Nullable(nullable) => {
                let inner = self.resolve_type_hint(&nullable.hint, scope);
                Type::Nullable(Box::new(inner))
            }
            Hint::Union(union) => {
                let left = self.resolve_type_hint(&union.left, scope);
                let right = self.resolve_type_hint(&union.right, scope);
                left.union_with(right)
            }
            Hint::Intersection(intersection) => {
                let left = self.resolve_type_hint(&intersection.left, scope);
                let right = self.resolve_type_hint(&intersection.right, scope);
                left.intersect_with(right)
            }
            Hint::Parenthesized(p) => self.resolve_type_hint(&p.hint, scope),
        }
    }

    /// Get text for a span
    fn get_span_text(&self, span: &mago_span::Span) -> &str {
        &self.source[span.start.offset as usize..span.end.offset as usize]
    }
}
