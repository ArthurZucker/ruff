use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::{self as ast, StmtClassDef};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::Violation;

#[derive(ViolationMetadata)]
pub(crate) struct ClassNamePrefix {
    expected: String,
}

impl Violation for ClassNamePrefix {
    #[derive_message_formats]
    fn message(&self) -> String {
        let Self { expected } = self;
        format!("Class name should start with `{expected}`")
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// RUF064
pub(crate) fn class_name_prefix(checker: &Checker, class_def: &StmtClassDef) {
    let Some(stem) = checker.path().file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let Some(model) = stem.strip_prefix("modular_") else {
        return;
    };
    let expected = capitalize(model);
    if !class_def.name.starts_with(&expected) {
        checker.report_diagnostic(
            ClassNamePrefix {
                expected: expected.clone(),
            },
            class_def.range(),
        );
    }
}
