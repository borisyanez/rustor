//! Remove unused use imports

use rustor_core::Edit;
use regex::Regex;
use std::collections::HashSet;
use crate::fixers::{Fixer, FixerConfig, edit_with_rule};

/// Removes use statements that are not referenced in the code
pub struct NoUnusedImportsFixer;

impl Fixer for NoUnusedImportsFixer {
    fn name(&self) -> &'static str {
        "no_unused_imports"
    }

    fn php_cs_fixer_name(&self) -> &'static str {
        "no_unused_imports"
    }

    fn description(&self) -> &'static str {
        "Remove unused use imports"
    }

    fn priority(&self) -> i32 {
        10  // Run after other import fixers
    }

    fn is_risky(&self) -> bool {
        true  // Removing imports could break code
    }

    fn check(&self, source: &str, _config: &FixerConfig) -> Vec<Edit> {
        let mut edits = Vec::new();

        // Collect all use statements
        let use_re = Regex::new(r"(?m)^[ \t]*(use\s+(?:function\s+|const\s+)?([^;]+));[ \t]*\n?").unwrap();

        let mut imports: Vec<ImportInfo> = Vec::new();

        for cap in use_re.captures_iter(source) {
            let full_match = cap.get(0).unwrap();
            let path = cap.get(2).unwrap().as_str();

            // Skip grouped imports (handled by single_import_per_statement)
            if path.contains('{') {
                continue;
            }

            // Extract the class/function/const name being imported
            let name = extract_import_name(path);

            imports.push(ImportInfo {
                start: full_match.start(),
                end: full_match.end(),
                name,
                full_path: path.to_string(),
            });
        }

        // Find all identifiers used in the code (after use statements)
        let used_names = find_used_identifiers(source);

        // Check each import
        for import in imports {
            if !is_import_used(&import.name, &used_names, source) {
                edits.push(edit_with_rule(
                    import.start,
                    import.end,
                    String::new(),
                    format!("Remove unused import '{}'", import.full_path),
                    "no_unused_imports",
                ));
            }
        }

        edits
    }
}

struct ImportInfo {
    start: usize,
    end: usize,
    name: String,
    full_path: String,
}

fn extract_import_name(path: &str) -> String {
    // Handle aliases: Foo\Bar as Baz -> Baz
    if path.contains(" as ") {
        return path.split(" as ")
            .last()
            .unwrap_or("")
            .trim()
            .to_string();
    }

    // Get the last part of the namespace: Foo\Bar\Baz -> Baz
    path.split('\\')
        .last()
        .unwrap_or(path)
        .trim()
        .to_string()
}

fn find_used_identifiers(source: &str) -> HashSet<String> {
    let mut used = HashSet::new();

    // Find class usages: new ClassName, ClassName::, extends ClassName, implements ClassName
    // Type hints: function foo(ClassName $x), : ClassName
    let patterns = [
        r"\bnew\s+([A-Z]\w*)",                     // new ClassName
        r"([A-Z]\w*)\s*::",                        // ClassName::
        r"\bextends\s+([A-Z]\w*)",                 // extends ClassName
        r"\bimplements\s+([A-Z][\w,\s\\]*)",       // implements ClassName, ...
        r"\binstanceof\s+([A-Z]\w*)",              // instanceof ClassName
        r":\s*\??([A-Z]\w*)",                      // : ClassName return type
        r"\(\s*\??([A-Z]\w*)\s+\$",                // (ClassName $param)
        r",\s*\??([A-Z]\w*)\s+\$",                 // , ClassName $param
        r"\bcatch\s*\(\s*([A-Z]\w*)",              // catch (Exception
        r"@var\s+([A-Z]\w*)",                      // @var ClassName
        r"@param\s+([A-Z]\w*)",                    // @param ClassName
        r"@return\s+([A-Z]\w*)",                   // @return ClassName
        r"@throws\s+([A-Z]\w*)",                   // @throws ClassName
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(source) {
                if let Some(m) = cap.get(1) {
                    // Handle implements with multiple classes
                    for name in m.as_str().split(',') {
                        let clean = name.trim().split('\\').last().unwrap_or(name.trim());
                        if !clean.is_empty() {
                            used.insert(clean.to_string());
                        }
                    }
                }
            }
        }
    }

    // Also find function calls that match imported functions
    let func_re = Regex::new(r"\b([a-z_]\w*)\s*\(").unwrap();
    for cap in func_re.captures_iter(source) {
        if let Some(m) = cap.get(1) {
            used.insert(m.as_str().to_string());
        }
    }

    // Find constant usages (uppercase identifiers)
    let const_re = Regex::new(r"\b([A-Z][A-Z0-9_]+)\b").unwrap();
    for cap in const_re.captures_iter(source) {
        if let Some(m) = cap.get(1) {
            used.insert(m.as_str().to_string());
        }
    }

    used
}

fn is_import_used(name: &str, used_names: &HashSet<String>, source: &str) -> bool {
    // Direct name match
    if used_names.contains(name) {
        return true;
    }

    // Check if the name appears in the source at all (simple check)
    // Skip the use statement itself by checking for usage patterns
    let patterns = [
        format!(r"\bnew\s+{}\b", regex::escape(name)),
        format!(r"\b{}\s*::", regex::escape(name)),
        format!(r"\bextends\s+{}\b", regex::escape(name)),
        format!(r"\bimplements\s+[^{{]*\b{}\b", regex::escape(name)),
        format!(r"\binstanceof\s+{}\b", regex::escape(name)),
        format!(r":\s*\??{}\b", regex::escape(name)),
        format!(r"\(\s*\??{}\s+\$", regex::escape(name)),
        format!(r",\s*\??{}\s+\$", regex::escape(name)),
        format!(r"\bcatch\s*\(\s*{}\b", regex::escape(name)),
        format!(r"@(?:var|param|return|throws)\s+{}\b", regex::escape(name)),
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(source) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<Edit> {
        NoUnusedImportsFixer.check(source, &FixerConfig::default())
    }

    #[test]
    fn test_used_import_unchanged() {
        let source = "<?php\n\nuse App\\Model;\n\n$m = new Model();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_unused_import() {
        let source = "<?php\n\nuse App\\Model;\nuse App\\View;\n\n$m = new Model();\n";
        let edits = check(source);

        assert_eq!(edits.len(), 1);
        assert!(edits[0].message.contains("View"));
    }

    #[test]
    fn test_used_in_type_hint() {
        let source = "<?php\n\nuse App\\Model;\n\nfunction foo(Model $m) {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_used_in_return_type() {
        let source = "<?php\n\nuse App\\Model;\n\nfunction foo(): Model {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_used_in_extends() {
        let source = "<?php\n\nuse App\\BaseModel;\n\nclass Model extends BaseModel {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_used_in_implements() {
        let source = "<?php\n\nuse App\\Contract;\n\nclass Model implements Contract {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_used_in_static_call() {
        let source = "<?php\n\nuse App\\Helper;\n\nHelper::doSomething();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_used_in_catch() {
        let source = "<?php\n\nuse App\\CustomException;\n\ntry {} catch (CustomException $e) {}\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_used_with_alias() {
        let source = "<?php\n\nuse App\\Model as M;\n\n$m = new M();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_used_in_docblock() {
        let source = "<?php\n\nuse App\\Model;\n\n/** @var Model $m */\n$m = foo();\n";
        let edits = check(source);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_multiple_unused() {
        let source = "<?php\n\nuse App\\A;\nuse App\\B;\nuse App\\C;\n\nclass Foo {}\n";
        let edits = check(source);
        assert_eq!(edits.len(), 3);
    }
}
