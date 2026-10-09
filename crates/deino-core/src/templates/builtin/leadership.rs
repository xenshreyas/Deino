//! Built-ins for boards and investors.

use deino_domain::SectionFormat::{Bullets, Table, Todo};

use super::{BuiltinRule, BuiltinTemplate, section};

/// A board meeting.
pub(super) const BOARD_MEETING: BuiltinTemplate = BuiltinTemplate {
    id: "board_meeting",
    version: 1,
    name: "Board meeting",
    description: "A board meeting: business update, key metrics, discussion and resolutions.",
    meeting_context: "A company presenting to its board. Record numbers exactly, attribute board members' questions and advice, and keep formal resolutions separate from informal guidance.",
    sections: &[
        section(
            "Business update",
            Bullets,
            "The leadership's summary of performance and progress since the last meeting.",
        ),
        section(
            "Key metrics",
            Table,
            "Metric, value and the change or comparison given; only figures stated in the meeting.",
        ),
        section(
            "Discussion",
            Bullets,
            "Each topic discussed, with board members' questions, concerns and advice attributed to them.",
        ),
        section(
            "Resolutions and approvals",
            Bullets,
            "Formal votes, approvals and resolutions, with the outcome. Write \"None\" if there were none.",
        ),
        section(
            "Action items",
            Todo,
            "Every commitment, with the owner and due date when stated.",
        ),
    ],
    rules: &[BuiltinRule::titled("board meeting", r"\bboard\b")],
};

/// Updating an existing investor.
pub(super) const INVESTOR_UPDATE: BuiltinTemplate = BuiltinTemplate {
    id: "investor_update",
    version: 1,
    name: "Investor update",
    description: "Updating an existing investor: progress, metrics, challenges, asks and their feedback.",
    meeting_context: "A founder or company updating an investor who already backs them. Capture metrics exactly, every ask made of the investor, and the investor's feedback and offers of help.",
    sections: &[
        section(
            "Highlights",
            Bullets,
            "Wins and progress since the last update.",
        ),
        section(
            "Metrics",
            Table,
            "Metric, value and change; only figures stated in the meeting.",
        ),
        section(
            "Challenges",
            Bullets,
            "Problems and risks shared, and how the company is addressing them.",
        ),
        section(
            "Asks",
            Bullets,
            "Specific help requested from the investor: introductions, hiring, advice, funding.",
        ),
        section(
            "Investor feedback",
            Bullets,
            "The investor's reactions, advice and offers of help, attributed.",
        ),
        section(
            "Follow-ups",
            Todo,
            "Commitments on both sides, with the owner and due date when stated.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "investor update",
        r"\binvestor (update|call|sync|check[\s-]?in)\b",
    )
    .external()],
};

/// Hearing a pitch, as the investor.
pub(super) const INVESTOR_PITCH: BuiltinTemplate = BuiltinTemplate {
    id: "investor_pitch",
    version: 1,
    name: "Investor pitch (VC side)",
    description: "Evaluating a startup pitch as an investor: team, market, traction and concerns.",
    meeting_context: "The user is the investor hearing a founder's pitch. Write the notes as the first draft of a deal memo: state what the founders claimed factually, and keep the investor's own concerns and questions clearly separate from those claims.",
    sections: &[
        section(
            "Company snapshot",
            Bullets,
            "What the company does, its stage and location, and the round size and valuation if mentioned.",
        ),
        section(
            "Team",
            Bullets,
            "The founders and key people, with the backgrounds and experience they described.",
        ),
        section(
            "Problem and product",
            Bullets,
            "The problem, who has it, and how the product solves it.",
        ),
        section(
            "Market and competition",
            Bullets,
            "Market size, target customers and competitors, as the founders described them.",
        ),
        section(
            "Traction",
            Table,
            "Metric, value and timeframe: revenue, growth, customers, retention, as stated.",
        ),
        section(
            "Concerns and open questions",
            Bullets,
            "Risks, inconsistencies and questions to follow up on.",
        ),
        section(
            "Next steps",
            Todo,
            "Diligence requests, intros and follow-up meetings, with the owner.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "pitch",
        r"\bpitch\b|\b(pre-?seed|seed|series [a-d]) round\b",
    )
    .external()],
};
