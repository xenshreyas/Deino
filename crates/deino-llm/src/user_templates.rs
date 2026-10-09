//! User-editable prompt templates (NOT-04): minijinja with strict undefined
//! variables and an allowlist (the anarlog `template-app` enhance validation,
//! MIT).
//!
//! A template is checked when it is saved, not when it runs: an unknown
//! variable or a syntax error is reported to the user with the names
//! involved, and rendering never silently drops a misspelt variable.

use std::collections::BTreeMap;

use minijinja::{Environment, UndefinedBehavior};

/// A user template failed to parse, validate or render.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TemplateError {
    /// The template is not valid Jinja.
    #[error("the template has a syntax error: {0}")]
    Syntax(String),
    /// The template uses variables Deino does not provide.
    #[error("the template uses unknown variables: {}", .0.join(", "))]
    UnknownVariables(Vec<String>),
    /// Rendering failed, for example a missing value.
    #[error("the template could not be filled in: {0}")]
    Render(String),
}

/// A validated user template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserTemplate {
    source: String,
}

impl UserTemplate {
    /// Parses `source` and checks that it uses only the `allowed` variables.
    pub fn parse(source: &str, allowed: &[&str]) -> Result<Self, TemplateError> {
        let environment = environment();
        let template = environment
            .template_from_str(source)
            .map_err(|error| TemplateError::Syntax(error.to_string()))?;
        let mut unknown: Vec<String> = template
            .undeclared_variables(false)
            .into_iter()
            .filter(|variable| !allowed.contains(&variable.as_str()))
            .collect();
        if !unknown.is_empty() {
            unknown.sort();
            return Err(TemplateError::UnknownVariables(unknown));
        }
        Ok(Self {
            source: source.to_owned(),
        })
    }

    /// Fills the template in. Every variable it uses must be in `values`.
    pub fn render(&self, values: &BTreeMap<&str, String>) -> Result<String, TemplateError> {
        let environment = environment();
        let template = environment
            .template_from_str(&self.source)
            .map_err(|error| TemplateError::Syntax(error.to_string()))?;
        template
            .render(values)
            .map_err(|error| TemplateError::Render(error.to_string()))
    }
}

fn environment() -> Environment<'static> {
    let mut environment = Environment::new();
    environment.set_undefined_behavior(UndefinedBehavior::Strict);
    environment
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALLOWED: [&str; 2] = ["title", "attendees"];

    #[test]
    fn allowed_variables_render() {
        let template =
            UserTemplate::parse("# {{ title }}\nWith {{ attendees }}", &ALLOWED).unwrap();
        let values = BTreeMap::from([
            ("title", "Pilot".to_owned()),
            ("attendees", "Ana".to_owned()),
        ]);

        assert_eq!(template.render(&values).unwrap(), "# Pilot\nWith Ana");
    }

    #[test]
    fn unknown_variables_are_rejected_at_parse_time_with_their_names() {
        let error = UserTemplate::parse("{{ heading }} {{ secret_key }} {{ title }}", &ALLOWED)
            .unwrap_err();

        assert_eq!(
            error,
            TemplateError::UnknownVariables(vec!["heading".into(), "secret_key".into()])
        );
    }

    #[test]
    fn a_missing_value_fails_instead_of_rendering_blank() {
        let template = UserTemplate::parse("{{ title }}", &ALLOWED).unwrap();

        let error = template.render(&BTreeMap::new()).unwrap_err();

        assert!(matches!(error, TemplateError::Render(_)));
    }

    #[test]
    fn syntax_errors_are_reported() {
        assert!(matches!(
            UserTemplate::parse("{% if %}", &ALLOWED),
            Err(TemplateError::Syntax(_))
        ));
    }
}
