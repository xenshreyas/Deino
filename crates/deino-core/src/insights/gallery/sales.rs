//! The gallery's sales and customer templates: discovery, feedback,
//! competitors, health, objections and the CRM deal summary.

use super::{Cardinality, FieldKind, InsightTemplate, ONE, Spec, always, choice, external, field};

pub(super) fn discovery_scorecard() -> InsightTemplate {
    Spec {
        key: "discovery_scorecard",
        name: "Discovery scorecard",
        summary: "Qualify a prospect: pain, budget, timeline, decision-maker and a 1–5 score.",
        description: "Score this sales discovery call. List the main questions the prospect asked, their core pain in their words, the budget they mentioned, their timeline, who makes the decision, and a 1–5 score of how well qualified they are (5: clear pain, budget, timeline and an engaged decision-maker). Example: Pain \"Dispatchers rebuild routes by hand every morning\", Budget 40000, Timeline \"Pilot in Q1\", Score 4.",
        fields: vec![
            field("Questions", FieldKind::Text),
            field("Pain", FieldKind::Text),
            field("Budget", FieldKind::Number),
            field("Timeline", FieldKind::Text),
            field("Decision-maker", FieldKind::Text),
            field("Score", FieldKind::Number),
        ],
        cardinality: ONE,
        apply_when: external(),
    }
    .build()
}

pub(super) fn customer_feedback() -> InsightTemplate {
    Spec {
        key: "customer_feedback",
        name: "Customer feedback",
        summary: "Feature requests, complaints and praise, each with a quote.",
        description: "Collect every piece of product feedback the customer gave: feature requests, complaints and praise. One answer per piece of feedback, summarised in a sentence, with the customer's own words as the quote. Example: Kind \"Feature request\", Feedback \"Wants recurring jobs to copy their notes\", Quote \"we retype the same notes every week\".",
        fields: vec![
            choice("Kind", &["Feature request", "Complaint", "Praise"]),
            field("Feedback", FieldKind::Text),
            field("Quote", FieldKind::Text),
        ],
        cardinality: Cardinality::Auto,
        apply_when: external(),
    }
    .build()
}

pub(super) fn competitor_mentions() -> InsightTemplate {
    Spec {
        key: "competitor_mentions",
        name: "Competitor mentions",
        summary: "Every competitor named, in what context and how it came across.",
        description: "Find every mention of a competing product or company. One answer per competitor, with what was said about it and whether it came across well or badly for them. Example: Competitor \"FieldPulse\", Context \"Used today; dispatchers find rescheduling slow\", Sentiment \"Negative\".",
        fields: vec![
            field("Competitor", FieldKind::Text),
            field("Context", FieldKind::Text),
            choice("Sentiment", &["Positive", "Neutral", "Negative"]),
        ],
        cardinality: Cardinality::Auto,
        apply_when: always(),
    }
    .build()
}

pub(super) fn health_signals() -> InsightTemplate {
    Spec {
        key: "health_signals",
        name: "Health signals",
        summary: "Red, yellow or green account health, with the signals behind it.",
        description: "Judge the health of this customer relationship. Red: churn language, escalations or a stalled rollout. Yellow: frustration or doubts that are being worked through. Green: engaged and expanding. Give the signals that decided it, and whether there is a real risk of churn.",
        fields: vec![
            choice("Health", &["Red", "Yellow", "Green"]),
            field("Signals", FieldKind::Text),
            field("Churn risk", FieldKind::Boolean),
        ],
        cardinality: ONE,
        apply_when: external(),
    }
    .build()
}

pub(super) fn objections() -> InsightTemplate {
    Spec {
        key: "objections",
        name: "Objections",
        summary: "Pushback raised, how it was answered, and whether it was settled.",
        description: "Find every objection or concern the other side raised (price, security, timing, fit). One answer per objection, with how it was answered and whether it was resolved in the meeting.",
        fields: vec![
            field("Objection", FieldKind::Text),
            field("Response", FieldKind::Text),
            field("Resolved", FieldKind::Boolean),
        ],
        cardinality: Cardinality::Auto,
        apply_when: external(),
    }
    .build()
}

pub(super) fn deal_summary() -> InsightTemplate {
    Spec {
        key: "deal_summary",
        name: "Deal summary for CRM",
        summary: "Company, stage, size, close date and next step, ready for a CRM.",
        description: "Summarise the deal discussed for a CRM record: the prospect company, the deal stage, its size in dollars if mentioned, the expected close date, the next step, and a two-sentence summary of where things stand.",
        fields: vec![
            field("Company", FieldKind::Text),
            choice(
                "Stage",
                &["Discovery", "Evaluation", "Proposal", "Negotiation", "Closed won", "Closed lost"],
            ),
            field("Deal size", FieldKind::Number),
            field("Close date", FieldKind::Date),
            field("Next step", FieldKind::Text),
            field("Summary", FieldKind::Text),
        ],
        cardinality: ONE,
        apply_when: external(),
    }
    .build()
}
