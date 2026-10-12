//! The shape the model answers an insight in, as typed wire values.
//!
//! These hold what the model said, unvalidated: a number may arrive as
//! `"$40k"`, a yes as `"yes"`, a field as a list. [`super::validate`] turns
//! them into typed [`InsightValue`](super::InsightValue)s or drops them. The
//! JSON schema the provider is held to is built per definition in
//! [`super::schema`]; these types accept anything that schema allows, and
//! more, so a loose model is repaired rather than failed.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The model's answer to one insight on one meeting.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct WireAnswer {
    /// One entry per answer; empty when the meeting holds none.
    #[serde(default)]
    pub answers: Vec<WireInstance>,
}

/// One answer, as the model wrote it.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct WireInstance {
    /// The answer of an insight without fields.
    #[serde(default)]
    pub value: Option<WireScalar>,
    /// The answer of an insight with fields, by field name.
    #[serde(default)]
    pub fields: BTreeMap<String, Option<WireScalar>>,
    /// The transcript line it came from (`s12`), or several.
    #[serde(default)]
    pub evidence: Option<WireEvidence>,
}

/// A JSON scalar, or a list of them where the model gave several.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum WireScalar {
    /// `true` or `false`.
    Bool(bool),
    /// Any JSON number.
    Number(f64),
    /// Any string.
    Text(String),
    /// A list (joined for text, otherwise its first usable item).
    List(Vec<WireScalar>),
}

/// One transcript reference or several.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum WireEvidence {
    /// `"s12"`.
    One(String),
    /// `["s12", "s14"]`.
    Many(Vec<String>),
}

impl WireEvidence {
    /// The references, in the model's order.
    pub fn references(&self) -> Vec<&str> {
        match self {
            Self::One(reference) => vec![reference.as_str()],
            Self::Many(references) => references.iter().map(String::as_str).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loose_answer_still_parses_into_wire_values() {
        let json = r#"{"answers":[
            {"fields":{"Budget":"$40k","Score":4,"Champion":true,"Pains":["slow","manual"],"Timeline":null},
             "evidence":"s12"},
            {"value":"How do you price seats?","evidence":["s3","s4"]},
            {}
        ]}"#;

        let answer: WireAnswer = serde_json::from_str(json).unwrap();

        let first = &answer.answers[0];
        assert_eq!(
            first.fields["Budget"],
            Some(WireScalar::Text("$40k".into()))
        );
        assert_eq!(first.fields["Score"], Some(WireScalar::Number(4.0)));
        assert_eq!(first.fields["Champion"], Some(WireScalar::Bool(true)));
        assert_eq!(first.fields["Timeline"], None);
        assert!(matches!(first.fields["Pains"], Some(WireScalar::List(_))));
        assert_eq!(first.evidence.as_ref().unwrap().references(), ["s12"]);
        assert_eq!(
            answer.answers[1].evidence.as_ref().unwrap().references(),
            ["s3", "s4"]
        );
        assert_eq!(answer.answers[2], WireInstance::default());
    }

    #[test]
    fn an_answer_without_the_list_is_empty() {
        let answer: WireAnswer = serde_json::from_str("{}").unwrap();
        assert!(answer.answers.is_empty());
    }
}
