use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::{self as ast, StmtClassDef};
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::Violation;

#[derive(ViolationMetadata)]
pub(crate) struct UnwrapInheritance {
    base: String,
}

impl Violation for UnwrapInheritance {
    #[derive_message_formats]
    fn message(&self) -> String {
        let Self { base } = self;
        format!("Class inherits from `{base}` which should be unwrapped")
    }
}

/// RUF062
pub(crate) fn unwrap_inheritance(checker: &Checker, class_def: &StmtClassDef) {
    let Some(arguments) = class_def.arguments.as_deref() else {
        return;
    };

    if checker.settings.ruff.unwrap_inheritance_modules.is_empty() {
        return;
    }

    for base in &*arguments.args {
        let Some(qualified) = checker.semantic().resolve_qualified_name(base) else {
            continue;
        };
        let full = qualified.to_string();
        if checker
            .settings
            .ruff
            .unwrap_inheritance_modules
            .iter()
            .any(|prefix| full.starts_with(prefix))
        {
            checker.report_diagnostic(
                UnwrapInheritance {
                    base: full,
                },
                base.range(),
            );
        }
    }
}
