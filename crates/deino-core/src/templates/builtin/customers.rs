//! Built-ins for selling to and supporting customers.

use deino_domain::SectionFormat::{Bullets, Paragraph, Table, Todo};

use super::{BuiltinRule, BuiltinTemplate, section};

/// A first call with a prospect.
pub(super) const SALES_DISCOVERY: BuiltinTemplate = BuiltinTemplate {
    id: "sales_discovery",
    version: 1,
    name: "Sales discovery",
    description: "A first call with a prospect: their situation, pain, buying process and fit.",
    meeting_context: "The user's team is selling to a prospect. Capture the prospect's own words about their pain and goals, the buying process (decision makers, budget, timeline, competitors) and anything that affects qualification. Keep the team's internal commentary out.",
    sections: &[
        section(
            "Prospect and attendees",
            Bullets,
            "Who the prospect is, their company, and the role of each attendee on their side.",
        ),
        section(
            "Current situation",
            Bullets,
            "How they work today, the tools they use, and what prompted the conversation now.",
        ),
        section(
            "Pain points",
            Bullets,
            "Problems they described, with their impact and short quotes where possible.",
        ),
        section(
            "Goals and success criteria",
            Bullets,
            "What they want to achieve and how they will judge a solution.",
        ),
        section(
            "Buying process",
            Table,
            "One row each for decision makers, budget, timeline, competitors and procurement, with what was learned. Write \"Unknown\" where nothing was said.",
        ),
        section(
            "Objections and concerns",
            Bullets,
            "Hesitations they raised and how each was answered; mark any left open.",
        ),
        section(
            "Next steps",
            Todo,
            "Next steps agreed on both sides, with the owner and date.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "external discovery call",
        r"\b(discovery|intro|introduction|introductory|exploratory|first call)\b",
    )
    .external()],
};

/// Demoing the product to a prospect.
pub(super) const SALES_DEMO: BuiltinTemplate = BuiltinTemplate {
    id: "sales_demo",
    version: 1,
    name: "Sales demo",
    description: "Showing the product to a prospect: what was shown, their reactions and next steps.",
    meeting_context: "A product demonstration to a prospect. Tie the prospect's reactions and questions to the features shown, and record objections and buying signals accurately.",
    sections: &[
        section(
            "Attendees and context",
            Bullets,
            "Who attended from the prospect and what they wanted to see.",
        ),
        section(
            "What was shown",
            Bullets,
            "Features and workflows demonstrated, in order.",
        ),
        section(
            "Reactions and questions",
            Bullets,
            "The prospect's questions, reactions and buying signals, attributed.",
        ),
        section(
            "Objections",
            Bullets,
            "Concerns raised and how they were answered; mark any left unanswered.",
        ),
        section(
            "Next steps",
            Todo,
            "Next steps agreed on both sides, with the owner and date.",
        ),
    ],
    rules: &[BuiltinRule::titled("external demo", r"\bdemo\b").external()],
};

/// Reviewing the sales pipeline.
pub(super) const PIPELINE_REVIEW: BuiltinTemplate = BuiltinTemplate {
    id: "pipeline_review",
    version: 1,
    name: "Pipeline review",
    description: "A sales team reviewing open deals: stages, risks and the forecast.",
    meeting_context: "A sales team going through its pipeline. Record each deal discussed with its stage, value and agreed next action, and flag deals that are slipping.",
    sections: &[
        section(
            "Forecast",
            Bullets,
            "The overall pipeline and forecast figures as stated, and what changed since the last review.",
        ),
        section(
            "Deals",
            Table,
            "Deal, owner, stage, value, expected close date and next step; only deals discussed.",
        ),
        section(
            "At-risk deals",
            Bullets,
            "Deals slipping or at risk, why, and the plan to recover each.",
        ),
        section(
            "Action items",
            Todo,
            "Every commitment, with the owner and due date when stated.",
        ),
    ],
    rules: &[
        BuiltinRule::titled("pipeline review", r"\b(pipeline|forecast|deal review)\b").internal(),
    ],
};

/// Meeting an existing customer.
pub(super) const CUSTOMER_CHECK_IN: BuiltinTemplate = BuiltinTemplate {
    id: "customer_check_in",
    version: 1,
    name: "Customer check-in / QBR",
    description: "A check-in or quarterly business review with an existing customer.",
    meeting_context: "An account team meeting a customer who already uses the product. Capture the customer's goals, satisfaction and problems, any expansion, renewal or churn signals, and every commitment made to them.",
    sections: &[
        section(
            "Account snapshot",
            Bullets,
            "Who attended from the customer, their goals, and how they use the product today.",
        ),
        section(
            "Results and value",
            Bullets,
            "Outcomes and metrics reviewed, as stated.",
        ),
        section(
            "Feedback and issues",
            Bullets,
            "Praise, complaints and open issues, attributed.",
        ),
        section(
            "Opportunities and risks",
            Bullets,
            "Expansion, renewal or churn signals, and what drives them.",
        ),
        section(
            "Commitments",
            Todo,
            "What each side agreed to deliver, with the owner and date.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "external check-in",
        r"\b(check[\s-]?in|qbr|business review|account review|follow[\s-]?up)\b",
    )
    .external()],
};

/// Helping a customer with a problem.
pub(super) const CUSTOMER_SUPPORT: BuiltinTemplate = BuiltinTemplate {
    id: "customer_support",
    version: 1,
    name: "Customer support call",
    description: "Helping a customer with a problem: the issue, troubleshooting, resolution and follow-ups.",
    meeting_context: "A support or escalation call with a customer. Be precise about the problem, the environment, the steps already tried and what was promised; engineering may read these notes.",
    sections: &[
        section(
            "Issue",
            Paragraph,
            "The problem in the customer's words, when it started and how it affects them.",
        ),
        section(
            "Details",
            Bullets,
            "Account, product area, versions, error messages and examples they mentioned.",
        ),
        section(
            "Troubleshooting",
            Bullets,
            "Steps taken before and during the call, in order, with their results.",
        ),
        section(
            "Resolution",
            Paragraph,
            "How it was resolved, or the workaround given and what remains open.",
        ),
        section(
            "Follow-ups",
            Todo,
            "Promises to the customer and internal escalations, with the owner and timing.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "external support call",
        r"\b(support|escalation|incident|outage|bug|ticket)\b",
    )
    .external()],
};
