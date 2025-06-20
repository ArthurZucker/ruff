use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::{self as ast, Alias, Identifier, Stmt, StmtImportFrom};
use ruff_text_size::{Ranged, TextRange};

use crate::checkers::ast::Checker;
use crate::{Edit, Fix, FixAvailability, Violation};

#[derive(ViolationMetadata)]
pub(crate) struct ExplicitRelativeImport;

impl Violation for ExplicitRelativeImport {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    #[derive_message_formats]
    fn message(&self) -> String {
        "Relative imports should specify a module".to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some("Specify module in import".to_string())
    }
}

pub(crate) fn explicit_relative_import(checker: &Checker, stmt: &Stmt, import: &StmtImportFrom) {
    if import.level == 0 || import.module.is_some() {
        return;
    }
    let Some(stem) = checker.path().file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let module_name = if let Some(rest) = stem.strip_prefix("modular_") {
        format!("modeling_{rest}")
    } else {
        stem.to_string()
    };

    let node = ast::StmtImportFrom {
        module: Some(Identifier::new(module_name.clone(), TextRange::default())),
        names: import.names.clone(),
        level: import.level,
        range: TextRange::default(),
        node_index: ast::AtomicNodeIndex::dummy(),
    };
    let mut diagnostic = checker.report_diagnostic(ExplicitRelativeImport, stmt.range());
    diagnostic.set_fix(Fix::safe_edit(Edit::range_replacement(
        checker.generator().stmt(&node.into()),
        stmt.range(),
    )));
}
