use rustc_hash::FxHashSet;
use ruff_macros::{derive_message_formats, ViolationMetadata};
use ruff_python_ast::{self as ast, helpers::is_docstring_stmt, whitespace::indentation, whitespace::trailing_lines_end, Stmt, StmtClassDef, StmtImportFrom};
use ruff_python_codegen::{Generator, Stylist};
use ruff_python_parser::parse_module;
use ruff_python_trivia::textwrap::indent;
use ruff_text_size::Ranged;
use std::{fs, path::{Path, PathBuf}};

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
                            // if !content.is_empty() {
                            //     let indent_str = indentation(class_def.start(), checker.source())
                            //         .unwrap_or("");
                            //     let indent_str = format!("{indent_str}{}", checker.stylist().indentation().as_str());
                            //     let content = indent(&content, &indent_str);
                            //     let at = if let Some(last) = class_def.body.last() {
                            //         trailing_lines_end(last, checker.source())
                            //     } else {
                            //         class_def.end()
                            //     };
                            //     edits.push(Edit::insertion(content.into_owned(), at));
                            // }
                        }
                    }
                }

                Ok(Fix::unsafe_edits(edits.remove(0), edits))
            });
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn rename_modeling(content: &str, from: &str, to: &str) -> String {
    let camel_from = capitalize(from);
    let camel_to = capitalize(to);
    content
        .replace(&camel_from, &camel_to)
        .replace(&from.to_ascii_uppercase(), &to.to_ascii_uppercase())
        .replace(from, to)
}

fn modeling_name_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.strip_prefix("modeling_"))
        .map(|s| s.to_string())
}

fn resolve_module_path(path: &Path, level: usize, module: Option<&str>) -> PathBuf {
    let mut result = path.parent().unwrap_or(path).to_path_buf();
    for _ in 0..level.saturating_sub(1) {
        result = result.parent().unwrap_or(&result).to_path_buf();
    }
    if let Some(module) = module {
        for part in module.split('.') {
            result.push(part);
        }
    }
    result.set_extension("py");
    result
}

fn load_statement_source(
    path: &Path,
    name: &str,
    rename_from: Option<&str>,
    rename_to: Option<&str>,
    stylist: &Stylist,
) -> Option<String> {
    let source = fs::read_to_string(path).ok()?;
    let parsed = parse_module(&source).ok()?;
    let generator = Generator::from(stylist);
    for stmt in parsed.into_suite() {
        match &stmt {
            Stmt::ClassDef(ast::StmtClassDef { name: ident, .. })
            | Stmt::FunctionDef(ast::StmtFunctionDef { name: ident, .. })
                if ident == name =>
            {
                let mut result = generator.stmt(&stmt);

                if let (Some(from), Some(to)) = (rename_from, rename_to) {
                    result = result.replace(from, to);
                }

                return Some(result);
            }
            _ => continue,
        }
    }
    None
}

pub(crate) fn unwrap_import_from(checker: &Checker, stmt: &Stmt, import: &StmtImportFrom) {
    let module = import.module.as_deref();
    if checker.settings.ruff.unwrap_inheritance_modules.is_empty() {
        return;
    }
    if module.is_none() {
        return;
    }
    let module = module.unwrap();
    if !checker
        .settings
        .ruff
        .unwrap_inheritance_modules
        .iter()
        .any(|prefix| module.starts_with(prefix))
    {
        return;
    }

    let from_name = module
        .rsplit_once('_')
        .map(|(_, name)| name)
        .unwrap_or(module);
    let to_name = modeling_name_from_path(checker.path());

    let mut edits = Vec::new();
    let mut names = Vec::new();
    for alias in &import.names {
        if &alias.name == "*" {
            continue;
        }
        let name = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
        let path = resolve_module_path(checker.path(), import.level as usize, Some(module));
        if let Some(to_name) = to_name.as_deref() {
            if let Some(content) = load_statement_source(&path, alias.name.as_str(), Some(from_name), Some(to_name), checker.stylist()) {
                edits.push(Edit::insertion(format!("{}{}", checker.stylist().line_ending().as_str(), content), stmt.end()));
            }
        } else if let Some(content) = load_statement_source(&path, alias.name.as_str(), None, None, checker.stylist()) {
            edits.push(Edit::insertion(format!("{}{}", checker.stylist().line_ending().as_str(), content), stmt.end()));
        }
        names.push(name);
    }

    if names.is_empty() {
        return;
    }

    if let Ok(remove_edit) = crate::fix::edits::remove_unused_imports(
        names.iter().copied(),
        stmt,
        checker.semantic().current_statement_parent(),
        checker.locator(),
        checker.stylist(),
        checker.indexer(),
    ) {
        checker.report_diagnostic(UnwrapInheritance { base: module.to_string() }, stmt.range()).try_set_fix(|| {
            Ok(Fix::unsafe_edits(remove_edit, edits))
        });
    }
}
