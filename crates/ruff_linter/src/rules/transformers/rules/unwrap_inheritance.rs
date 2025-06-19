use rustc_hash::FxHashSet;
use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::{self as ast, helpers::is_docstring_stmt, whitespace::trailing_lines_end, whitespace::indentation, Stmt, StmtClassDef};
use ruff_python_trivia::textwrap::indent;
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
use crate::{Edit, Fix, Violation};

#[derive(ViolationMetadata)]
pub(crate) struct UnwrapInheritance {
    base: String,
}

fn stmt_name(stmt: &Stmt) -> Option<&str> {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) => Some(name.as_str()),
        Stmt::ClassDef(ast::StmtClassDef { name, .. }) => Some(name.as_str()),
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            if let [expr] = targets.as_slice() {
                expr.as_name_expr().map(|name| name.id.as_str())
            } else {
                None
            }
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => {
            target.as_name_expr().map(|name| name.id.as_str())
        }
        _ => None,
    }
}

fn member_names(class_def: &StmtClassDef) -> FxHashSet<&str> {
    class_def
        .body
        .iter()
        .filter_map(stmt_name)
        .collect()
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
            let mut diagnostic = checker.report_diagnostic(
                UnwrapInheritance {
                    base: full.clone(),
                },
                base.range(),
            );

            diagnostic.try_set_fix(|| {
                let mut edits = Vec::new();
                let source = checker.locator().contents();
                edits.push(crate::fix::edits::remove_argument(
                    base,
                    arguments,
                    crate::fix::edits::Parentheses::Preserve,
                    source,
                )?);

                if let Some(binding_id) = checker.semantic().lookup_attribute(base) {
                    if let ruff_python_semantic::BindingKind::ClassDefinition(..) =
                        checker.semantic().binding(binding_id).kind
                    {
                        if let Some(Stmt::ClassDef(base_def)) =
                            checker.semantic().binding(binding_id).statement(checker.semantic())
                        {
                            let existing = member_names(class_def);
                            let line_ending = checker.stylist().line_ending().as_str();
                            let mut content = String::new();
                            for stmt in &base_def.body {
                                if is_docstring_stmt(stmt) {
                                    continue;
                                }
                                if let Some(name) = stmt_name(stmt) {
                                    if existing.contains(name) {
                                        continue;
                                    }
                                }
                                content.push_str(&checker.generator().stmt(stmt));
                                if !content.ends_with(line_ending) {
                                    content.push_str(line_ending);
                                }
                            }
                            if !content.is_empty() {
                                let indent_str = indentation(class_def.start(), checker.source())
                                    .unwrap_or("");
                                let indent_str = format!("{indent_str}{}", checker.stylist().indentation());
                                let content = indent(&content, &indent_str);
                                let at = if let Some(last) = class_def.body.last() {
                                    trailing_lines_end(last, checker.source())
                                } else {
                                    class_def.end()
                                };
                                edits.push(Edit::insertion(content.into_owned(), at));
                            }
                        }
                    }
                }

                Ok(Fix::unsafe_edits(edits.remove(0), edits))
            });
        }
    }
}
