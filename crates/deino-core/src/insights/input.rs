//! The rules a definition must keep, applied to what the user typed before
//! anything is stored: trimmed names, unique field names and options, sane
//! counts, conditions in canonical form.

use super::{ApplyWhen, Cardinality, FieldKind, InsightError, InsightField, InsightInput};

/// The longest insight name, in characters.
pub const MAX_NAME_CHARS: usize = 80;
/// The longest description (the prompt), in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 4_000;
/// The most fields an insight has.
pub const MAX_FIELDS: usize = 12;
/// The longest field name, in characters.
pub const MAX_FIELD_NAME_CHARS: usize = 60;
/// The most options an enum field has.
pub const MAX_OPTIONS: usize = 20;
/// The longest enum option, in characters.
pub const MAX_OPTION_CHARS: usize = 60;
/// The highest fixed answer count.
pub const MAX_FIXED_ANSWERS: u8 = 20;

impl InsightInput {
    /// The input in canonical form, or the first rule it breaks.
    pub fn validated(self) -> Result<Self, InsightError> {
        Ok(Self {
            name: bounded(&self.name, MAX_NAME_CHARS, "name")?,
            description: bounded(&self.description, MAX_DESCRIPTION_CHARS, "description")?,
            fields: fields(self.fields)?,
            cardinality: cardinality(self.cardinality)?,
            apply_when: apply_when(self.apply_when)?,
        })
    }
}

/// `text` trimmed, refused when empty or longer than `max` characters.
fn bounded(text: &str, max: usize, field: &'static str) -> Result<String, InsightError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(InsightError::invalid(field, "cannot be empty"));
    }
    if text.chars().count() > max {
        return Err(InsightError::invalid(field, "is too long"));
    }
    Ok(text.to_owned())
}

fn fields(fields: Vec<InsightField>) -> Result<Vec<InsightField>, InsightError> {
    if fields.len() > MAX_FIELDS {
        return Err(InsightError::invalid("fields", "are too many"));
    }
    let fields: Vec<InsightField> = fields.into_iter().map(field).collect::<Result<_, _>>()?;
    if has_repeats(fields.iter().map(|field| field.name.as_str())) {
        return Err(InsightError::invalid("field names", "must differ"));
    }
    Ok(fields)
}

fn field(field: InsightField) -> Result<InsightField, InsightError> {
    let name = bounded(&field.name, MAX_FIELD_NAME_CHARS, "field name")?;
    let kind = match field.kind {
        FieldKind::Enum { options } => FieldKind::Enum {
            options: options_of(&options)?,
        },
        kind => kind,
    };
    Ok(InsightField { name, kind })
}

fn options_of(options: &[String]) -> Result<Vec<String>, InsightError> {
    if options.is_empty() || options.len() > MAX_OPTIONS {
        return Err(InsightError::invalid("options", "must number 1 to 20"));
    }
    let options: Vec<String> = options
        .iter()
        .map(|option| bounded(option, MAX_OPTION_CHARS, "option"))
        .collect::<Result<_, _>>()?;
    if has_repeats(options.iter().map(String::as_str)) {
        return Err(InsightError::invalid("options", "must differ"));
    }
    Ok(options)
}

/// True when two of `names` are equal ignoring case.
fn has_repeats<'a>(names: impl Iterator<Item = &'a str>) -> bool {
    let mut seen: Vec<String> = Vec::new();
    for name in names {
        let key = name.to_lowercase();
        if seen.contains(&key) {
            return true;
        }
        seen.push(key);
    }
    false
}

fn cardinality(cardinality: Cardinality) -> Result<Cardinality, InsightError> {
    match cardinality {
        Cardinality::Fixed { count } if count == 0 || count > MAX_FIXED_ANSWERS => {
            Err(InsightError::invalid("answer count", "must be 1 to 20"))
        }
        cardinality => Ok(cardinality),
    }
}

fn apply_when(apply_when: ApplyWhen) -> Result<ApplyWhen, InsightError> {
    match apply_when {
        ApplyWhen::Automatic { conditions } => Ok(ApplyWhen::Automatic {
            conditions: conditions.normalized()?,
        }),
        ApplyWhen::Manual => Ok(ApplyWhen::Manual),
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{Condition, ConditionGroups};

    use super::*;

    fn input() -> InsightInput {
        InsightInput {
            name: "  Health signals ".into(),
            description: " How healthy is the account? ".into(),
            fields: vec![InsightField {
                name: " Health ".into(),
                kind: FieldKind::Enum {
                    options: vec![" Red".into(), "Yellow ".into(), "Green".into()],
                },
            }],
            cardinality: Cardinality::Fixed { count: 1 },
            apply_when: ApplyWhen::Automatic {
                conditions: ConditionGroups(vec![vec![Condition::AttendeeDomain {
                    domain: "@Acme.com".into(),
                }]]),
            },
        }
    }

    #[test]
    fn valid_input_is_trimmed_into_canonical_form() {
        let input = input().validated().unwrap();

        assert_eq!(input.name, "Health signals");
        assert_eq!(input.description, "How healthy is the account?");
        assert_eq!(input.fields[0].name, "Health");
        assert_eq!(
            input.fields[0].kind,
            FieldKind::Enum {
                options: vec!["Red".into(), "Yellow".into(), "Green".into()]
            }
        );
        assert_eq!(
            input.apply_when,
            ApplyWhen::Automatic {
                conditions: ConditionGroups(vec![vec![Condition::AttendeeDomain {
                    domain: "acme.com".into()
                }]])
            }
        );
    }

    /// The valid input with one change.
    fn with(change: impl FnOnce(&mut InsightInput)) -> InsightInput {
        let mut input = input();
        change(&mut input);
        input
    }

    fn text(name: &str) -> InsightField {
        InsightField {
            name: name.into(),
            kind: FieldKind::Text,
        }
    }

    fn enum_of(options: &[&str]) -> InsightField {
        let options = options.iter().map(|option| (*option).to_owned()).collect();
        InsightField {
            name: "Health".into(),
            kind: FieldKind::Enum { options },
        }
    }

    #[test]
    fn each_rule_names_the_field_it_protects() {
        let fixed = |count| Cardinality::Fixed { count };
        let cases: Vec<(InsightInput, &str)> = vec![
            (with(|i| i.name = " ".into()), "name cannot be empty"),
            (with(|i| i.name = "x".repeat(81)), "name is too long"),
            (
                with(|i| i.description = String::new()),
                "description cannot be empty",
            ),
            (
                with(|i| i.fields = vec![text("Pain"), text("pain ")]),
                "field names must differ",
            ),
            (
                with(|i| i.fields = vec![text("")]),
                "field name cannot be empty",
            ),
            (
                with(|i| i.fields = vec![text("x"); 13]),
                "fields are too many",
            ),
            (
                with(|i| i.fields = vec![enum_of(&[])]),
                "options must number 1 to 20",
            ),
            (
                with(|i| i.fields = vec![enum_of(&["Red", "red"])]),
                "options must differ",
            ),
            (
                with(|i| i.cardinality = fixed(0)),
                "answer count must be 1 to 20",
            ),
            (
                with(|i| i.cardinality = fixed(21)),
                "answer count must be 1 to 20",
            ),
        ];

        for (input, message) in cases {
            assert_eq!(input.validated().unwrap_err().to_string(), message);
        }
    }

    #[test]
    fn an_unmatchable_condition_is_refused() {
        let input = InsightInput {
            apply_when: ApplyWhen::Automatic {
                conditions: ConditionGroups(vec![vec![Condition::TitleContains {
                    text: " ".into(),
                }]]),
            },
            ..input()
        };

        assert!(matches!(input.validated(), Err(InsightError::Condition(_))));
    }
}
