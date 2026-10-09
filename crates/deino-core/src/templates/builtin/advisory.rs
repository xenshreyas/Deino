//! Built-ins for professional services and outside relationships: clinical
//! sessions, legal consultations, partners and vendors.

use deino_domain::SectionFormat::{Bullets, Paragraph, Table, Todo};

use super::{BuiltinRule, BuiltinTemplate, section};

/// A clinical session note (the pre-NOT-04 "Psychiatric session" template).
pub(super) const THERAPY_SOAP: BuiltinTemplate = BuiltinTemplate {
    id: "therapy_soap",
    version: 1,
    name: "Therapy / SOAP",
    description: "A clinical session note in SOAP form: subjective, objective, assessment and plan.",
    meeting_context: "A clinician's progress note. Use neutral clinical language and record only what was said or observed in the session; never invent diagnoses, doses or risk findings, and mark anything uncertain for the clinician to verify. The clinician reviews and signs every note.",
    sections: &[
        section(
            "Session summary",
            Paragraph,
            "One sentence with the main takeaway, then a short summary of the session.",
        ),
        section(
            "Subjective",
            Paragraph,
            "The client's reported mood, symptoms, sleep, appetite, stressors and events, in their own words where useful.",
        ),
        section(
            "Objective",
            Paragraph,
            "Observations during the session: affect, speech, behaviour, orientation and engagement.",
        ),
        section(
            "Assessment",
            Paragraph,
            "The clinician's stated impressions, progress towards treatment goals, and any risk discussed (suicidal or homicidal ideation). Add no impression that was not stated.",
        ),
        section(
            "Plan",
            Bullets,
            "Interventions, homework, referrals, medication changes and follow-up, as discussed.",
        ),
        section(
            "Medications",
            Table,
            "Medication, dose, change and notes, only as stated. Write \"Not discussed\" if medication did not come up.",
        ),
        section(
            "Safety",
            Bullets,
            "Safety planning and risk-management steps. Write \"Not discussed\" if none.",
        ),
        section(
            "Next session",
            Paragraph,
            "The date, time and format of the next appointment, if agreed.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "clinical session",
        r"\b(therapy|counsell?ing|psychiatr\w*|session note|soap note)\b",
    )],
};

/// A consultation with a lawyer.
pub(super) const LEGAL_CONSULTATION: BuiltinTemplate = BuiltinTemplate {
    id: "legal_consultation",
    version: 1,
    name: "Legal consultation",
    description: "A consultation with a lawyer or client: facts, issues, advice, deadlines and next steps.",
    meeting_context: "A legal consultation. Keep facts separate from legal analysis, record advice exactly as given with its caveats and assumptions, and list deadlines precisely. These notes are a record, not legal advice.",
    sections: &[
        section(
            "Matter",
            Paragraph,
            "The matter, the parties involved and the client's goal.",
        ),
        section(
            "Facts",
            Bullets,
            "Relevant facts and documents described, with dates.",
        ),
        section(
            "Issues",
            Bullets,
            "The legal questions and risks identified.",
        ),
        section(
            "Advice",
            Bullets,
            "Advice and options given, with caveats and assumptions, attributed.",
        ),
        section(
            "Deadlines",
            Table,
            "Deadline, date, what is due and who owns it.",
        ),
        section(
            "Next steps",
            Todo,
            "Documents to gather, filings and follow-ups, with the owner.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "legal consultation",
        r"\b(legal|lawyer|attorney|counsel|contract review)\b",
    )],
};

/// Exploring or running a partnership.
pub(super) const PARTNERSHIP_CALL: BuiltinTemplate = BuiltinTemplate {
    id: "partnership_call",
    version: 1,
    name: "Partnership call",
    description: "Exploring or managing a partnership: mutual goals, proposed terms, concerns and next steps.",
    meeting_context: "Two organisations discussing working together. Capture what each side wants and offers, the terms proposed, and how the partnership would work in practice.",
    sections: &[
        section(
            "Partner overview",
            Bullets,
            "Who the partner is, who attended and their roles.",
        ),
        section(
            "Mutual goals",
            Bullets,
            "What each side wants from the partnership.",
        ),
        section(
            "Proposal and terms",
            Bullets,
            "The partnership model, commercial terms, exclusivity or integration plans discussed.",
        ),
        section(
            "Concerns",
            Bullets,
            "Open questions and risks on either side.",
        ),
        section(
            "Next steps",
            Todo,
            "Next steps agreed on both sides, with the owner and date.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "external partnership call",
        r"\b(partner(ship)?s?|alliance|co-?marketing|reseller)\b",
    )
    .external()],
};

/// Assessing a vendor.
pub(super) const VENDOR_EVALUATION: BuiltinTemplate = BuiltinTemplate {
    id: "vendor_evaluation",
    version: 1,
    name: "Vendor evaluation",
    description: "Evaluating a vendor or tool: requirements fit, pricing, risks and next steps.",
    meeting_context: "The user's team is assessing a vendor. Compare what the vendor claimed against the team's requirements, record pricing and terms exactly, and note anything to verify.",
    sections: &[
        section(
            "Vendor and offering",
            Bullets,
            "Who the vendor is, who attended, and the product or service discussed.",
        ),
        section(
            "Requirements fit",
            Table,
            "Requirement, the vendor's answer, and the fit: meets, partial or gap.",
        ),
        section(
            "Pricing and terms",
            Bullets,
            "Pricing, contract length, SLAs and terms, as stated.",
        ),
        section(
            "Security and risks",
            Bullets,
            "Security, compliance, lock-in and other risks raised.",
        ),
        section(
            "Next steps",
            Todo,
            "Trials, references, documents requested and the decision timeline, with the owner.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "external vendor evaluation",
        r"\b(vendor|procurement|rfp|evaluation|pricing)\b",
    )
    .external()],
};
