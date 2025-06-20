//! Transformers-specific rules.
mod fixes;
pub(crate) mod rules;
pub mod settings;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use test_case::test_case;

    use crate::assert_messages;
    use crate::registry::Rule;
    use crate::settings::LinterSettings;
    use crate::test::test_path;

    #[test_case(Rule::ExplicitRelativeImport, Path::new("RUF063.py"))]
    fn rules(rule_code: Rule, path: &Path) -> Result<()> {
        let snapshot = format!("{}_{}", rule_code.noqa_code(), path.to_string_lossy());
        let diagnostics = test_path(Path::new("ruff").join(path).as_path(), &LinterSettings::for_rule(rule_code))?;
        assert_messages!(snapshot, diagnostics);
        Ok(())
    }
}
