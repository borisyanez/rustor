//! Native type declaration casing

use rustor_core::Edit;
use regex::Regex;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

pub struct NativeTypeDeclarationCasingFixer;

impl Fixer for NativeTypeDeclarationCasingFixer {
    fn name(&self) -> &'static str { "native_type_declaration_casing" }
    fn php_cs_fixer_name(&self) -> &'static str { "native_type_declaration_casing" }
    fn description(&self) -> &'static str { "Lowercase native types" }
    fn priority(&self) -> i32 { 40 }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Match type context: after ( or : or | or ? with optional space before type
        // Types that need to be lowercase (capture non-lowercase variants)
        let types = [
            ("Int", "int"), ("INT", "int"), ("String", "string"), ("STRING", "string"),
            ("Bool", "bool"), ("BOOL", "bool"), ("Float", "float"), ("FLOAT", "float"),
            ("Array", "array"), ("ARRAY", "array"), ("Object", "object"), ("OBJECT", "object"),
            ("Mixed", "mixed"), ("MIXED", "mixed"), ("Void", "void"), ("VOID", "void"),
            ("Null", "null"), ("NULL", "null"), ("False", "false"), ("FALSE", "false"),
            ("True", "true"), ("TRUE", "true"), ("Iterable", "iterable"), ("ITERABLE", "iterable"),
            ("Callable", "callable"), ("CALLABLE", "callable"), ("Never", "never"), ("NEVER", "never"),
        ];

        for (from, to) in &types {
            // Match type in type context: (Type or :Type or |Type or ?Type
            // Followed by space, |, ), comma, {, or end of word
            let re = Regex::new(&format!(r"([(:|?]\s*){}\b", from)).unwrap();

            for cap in re.captures_iter(source) {
                let full = cap.get(0).unwrap();
                let prefix = cap.get(1).unwrap().as_str();

                if is_in_string(&source[..full.start()]) { continue; }

                edits.push(edit_with_rule(
                    full.start(), full.end(),
                    format!("{}{}", prefix, to),
                    "Use lowercase native type".to_string(),
                    "native_type_declaration_casing",
                ));
            }
        }

        edits
    }
}

fn is_in_string(before: &str) -> bool {
    let (mut s, mut d, mut p) = (false, false, '\0');
    for c in before.chars() {
        if c == '\'' && p != '\\' && !d { s = !s; }
        if c == '"' && p != '\\' && !s { d = !d; }
        p = c;
    }
    s || d
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_uppercase_int() {
        let edits = NativeTypeDeclarationCasingFixer.check("function f(Int )", &FixerConfig::default());
        assert!(!edits.is_empty());
    }

    #[test]
    fn test_lowercase_int() {
        let edits = NativeTypeDeclarationCasingFixer.check("function f(int )", &FixerConfig::default());
        assert!(edits.is_empty());
    }

    #[test]
    fn test_return_type() {
        let edits = NativeTypeDeclarationCasingFixer.check("function f(): Int {}", &FixerConfig::default());
        assert!(!edits.is_empty());
    }
}
