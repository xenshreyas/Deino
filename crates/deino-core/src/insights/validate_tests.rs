//! Typed field validation, tabled: what each field type accepts, repairs
//! and drops, and how whole answers are kept, merged and cut.

use std::collections::BTreeMap;

use super::*;
use crate::insights::wire::WireEvidence;

fn text(value: &str) -> WireScalar {
    WireScalar::Text(value.into())
}

fn number(value: f64) -> WireScalar {
    WireScalar::Number(value)
}

fn day(year: i32, month: u32, date: u32) -> FieldValue {
    FieldValue::Date(NaiveDate::from_ymd_opt(year, month, date).unwrap())
}

fn health() -> FieldKind {
    FieldKind::Enum {
        options: vec!["Red".into(), "Yellow".into(), "Green".into()],
    }
}

#[test]
fn each_field_type_accepts_repairs_or_drops_values() {
    let cases: Vec<(FieldKind, WireScalar, Option<FieldValue>)> = vec![
        // Text: trimmed; numbers and yes/no written out; lists joined; blank dropped.
        (
            FieldKind::Text,
            text("  Okta SSO "),
            Some(FieldValue::Text("Okta SSO".into())),
        ),
        (
            FieldKind::Text,
            number(120.0),
            Some(FieldValue::Text("120".into())),
        ),
        (
            FieldKind::Text,
            WireScalar::Bool(true),
            Some(FieldValue::Text("Yes".into())),
        ),
        (
            FieldKind::Text,
            WireScalar::List(vec![text("slow"), text(""), text("manual")]),
            Some(FieldValue::Text("slow; manual".into())),
        ),
        (FieldKind::Text, text("   "), None),
        // Number: currency, separators, suffixes and percents repaired.
        (
            FieldKind::Number,
            number(4.0),
            Some(FieldValue::Number(4.0)),
        ),
        (
            FieldKind::Number,
            text("$40,000"),
            Some(FieldValue::Number(40_000.0)),
        ),
        (
            FieldKind::Number,
            text("40k"),
            Some(FieldValue::Number(40_000.0)),
        ),
        (
            FieldKind::Number,
            text("1.5M"),
            Some(FieldValue::Number(1_500_000.0)),
        ),
        (
            FieldKind::Number,
            text("12.5%"),
            Some(FieldValue::Number(12.5)),
        ),
        (
            FieldKind::Number,
            text("~120"),
            Some(FieldValue::Number(120.0)),
        ),
        (FieldKind::Number, text("four"), None),
        (FieldKind::Number, text("4/5"), None),
        (FieldKind::Number, WireScalar::Bool(true), None),
        (FieldKind::Number, number(f64::INFINITY), None),
        // Boolean: booleans, 1/0 and yes/no words.
        (
            FieldKind::Boolean,
            WireScalar::Bool(false),
            Some(FieldValue::Boolean(false)),
        ),
        (
            FieldKind::Boolean,
            text("Yes."),
            Some(FieldValue::Boolean(true)),
        ),
        (
            FieldKind::Boolean,
            text("FALSE"),
            Some(FieldValue::Boolean(false)),
        ),
        (
            FieldKind::Boolean,
            number(1.0),
            Some(FieldValue::Boolean(true)),
        ),
        (FieldKind::Boolean, number(2.0), None),
        (FieldKind::Boolean, text("maybe"), None),
        // Date: ISO, ISO date-times and written-out dates.
        (FieldKind::Date, text("2026-10-08"), Some(day(2026, 10, 8))),
        (
            FieldKind::Date,
            text("2026-10-08T09:30:00Z"),
            Some(day(2026, 10, 8)),
        ),
        (
            FieldKind::Date,
            text("October 8, 2026"),
            Some(day(2026, 10, 8)),
        ),
        (FieldKind::Date, text("8 Oct 2026"), Some(day(2026, 10, 8))),
        (FieldKind::Date, text("next Friday"), None),
        (FieldKind::Date, text("2026-02-30"), None),
        (FieldKind::Date, number(20_261_008.0), None),
        // Enum: the option as spelled, matched ignoring case and punctuation.
        (
            health(),
            text("Green"),
            Some(FieldValue::Choice("Green".into())),
        ),
        (
            health(),
            text("yellow"),
            Some(FieldValue::Choice("Yellow".into())),
        ),
        (
            health(),
            text(" red. "),
            Some(FieldValue::Choice("Red".into())),
        ),
        (health(), text("Amber"), None),
        (health(), number(1.0), None),
    ];

    for (kind, raw, expected) in cases {
        assert_eq!(coerce(&kind, &raw), expected, "{kind:?} {raw:?}");
    }
}

#[test]
fn long_text_is_cut_to_the_limit() {
    let long = "x".repeat(MAX_VALUE_CHARS + 10);

    let Some(FieldValue::Text(kept)) = coerce(&FieldKind::Text, &text(&long)) else {
        panic!("long text is kept");
    };

    assert_eq!(kept.chars().count(), MAX_VALUE_CHARS);
}

fn field(name: &str, kind: FieldKind) -> InsightField {
    InsightField {
        name: name.into(),
        kind,
    }
}

fn scorecard() -> Vec<InsightField> {
    vec![
        field("Budget", FieldKind::Number),
        field("Health", health()),
    ]
}

fn instance(fields: &[(&str, Option<WireScalar>)], evidence: &str) -> WireInstance {
    WireInstance {
        value: None,
        fields: fields
            .iter()
            .map(|(name, raw)| ((*name).to_owned(), raw.clone()))
            .collect::<BTreeMap<_, _>>(),
        evidence: Some(WireEvidence::One(evidence.into())),
    }
}

#[test]
fn field_answers_keep_valid_values_and_leave_invalid_ones_empty() {
    let wire = WireAnswer {
        answers: vec![instance(
            &[
                ("budget ", Some(text("$40k"))),
                ("Health", Some(text("Amber"))),
            ],
            "s12",
        )],
    };

    let answers = validate(&wire, &scorecard(), Cardinality::Auto);

    assert_eq!(
        answers,
        [Validated {
            value: InsightValue::Fields {
                fields: vec![
                    FieldEntry {
                        name: "Budget".into(),
                        value: Some(FieldValue::Number(40_000.0)),
                    },
                    FieldEntry {
                        name: "Health".into(),
                        value: None,
                    },
                ],
            },
            references: vec!["s12".into()],
        }]
    );
}

#[test]
fn answers_with_nothing_valid_and_repeats_are_dropped_and_the_rest_cut_to_the_count() {
    let budget = |raw: &str| instance(&[("Budget", Some(text(raw)))], "s1");
    let wire = WireAnswer {
        answers: vec![
            instance(&[("Budget", Some(text("lots"))), ("Health", None)], "s1"),
            budget("10"),
            budget("10"),
            budget("20"),
            budget("30"),
        ],
    };

    let auto = validate(&wire, &scorecard(), Cardinality::Auto);
    let two = validate(&wire, &scorecard(), Cardinality::Fixed { count: 2 });

    let budgets = |answers: &[Validated]| -> Vec<Option<FieldValue>> {
        answers
            .iter()
            .map(|answer| answer.value.field("Budget").cloned())
            .collect()
    };
    assert_eq!(
        budgets(&auto),
        [10.0, 20.0, 30.0].map(|value| Some(FieldValue::Number(value)))
    );
    assert_eq!(
        budgets(&two),
        [10.0, 20.0].map(|value| Some(FieldValue::Number(value)))
    );
}

#[test]
fn free_text_answers_need_a_value() {
    let wire = WireAnswer {
        answers: vec![
            WireInstance {
                value: Some(text("How is pricing structured?")),
                ..WireInstance::default()
            },
            WireInstance {
                value: Some(text("  ")),
                ..WireInstance::default()
            },
            WireInstance::default(),
        ],
    };

    let answers = validate(&wire, &[], Cardinality::Auto);

    assert_eq!(
        answers,
        [Validated {
            value: InsightValue::Text {
                text: "How is pricing structured?".into()
            },
            references: Vec::new(),
        }]
    );
}
