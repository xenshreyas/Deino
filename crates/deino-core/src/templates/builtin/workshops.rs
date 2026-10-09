//! Built-ins for working sessions and broadcasts: Circleback-style notes,
//! design critiques, brainstorms and all-hands.

use deino_domain::SectionFormat::{Bullets, Paragraph, Todo};

use super::{BuiltinRule, BuiltinTemplate, section};

/// Circleback's default note structure (`docs/research/circleback.md` §5):
/// action items first, a short overview, then one section per topic.
pub(super) const CIRCLEBACK_STYLE: BuiltinTemplate = BuiltinTemplate {
    id: "circleback_style",
    version: 1,
    name: "Circleback style",
    description: "Action items first, a three-to-six bullet overview, then one section per topic discussed.",
    meeting_context: "Notes for someone who missed the meeting: they should get the context, decisions and next steps at a glance. Attribute points to people by name (\"Ali demonstrated the new flow\"). There is no separate decisions section; record decisions inside the overview and the topic sections.",
    sections: &[
        section(
            "Action Items",
            Todo,
            "Every task someone took on: the owner's name, the task and the due date when stated.",
        ),
        section(
            "Overview",
            Bullets,
            "Three to six bullets summarising the whole meeting: its purpose, the key outcomes and the decisions made.",
        ),
        section(
            "Topics",
            Bullets,
            "One subsection per discussion topic, in the order discussed: a short topic title as a third-level heading, then detailed bullets that attribute each point to the person who made it.",
        ),
    ],
    rules: &[],
};

/// Feedback on a design.
pub(super) const DESIGN_CRITIQUE: BuiltinTemplate = BuiltinTemplate {
    id: "design_critique",
    version: 1,
    name: "Design critique",
    description: "Feedback on a design: what was shown, the reactions and the agreed changes.",
    meeting_context: "Designers presenting work for critique. Separate what was shown from the feedback, tie each piece of feedback to the part of the design it concerns, and say which changes were agreed rather than merely suggested.",
    sections: &[
        section(
            "What was presented",
            Bullets,
            "The designs, flows or prototypes shown, and the questions the presenter asked the group.",
        ),
        section(
            "Feedback",
            Bullets,
            "Grouped by screen or area. For each point, who raised it and whether it is a problem, a question or a suggestion.",
        ),
        section(
            "Agreed changes",
            Todo,
            "Changes the presenter agreed to make, with the owner.",
        ),
        section(
            "Open questions",
            Bullets,
            "Disagreements and questions left unresolved.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "design critique",
        r"\bdesign (crit|critique|review)\b|\bcrit\b",
    )],
};

/// Generating ideas.
pub(super) const BRAINSTORM: BuiltinTemplate = BuiltinTemplate {
    id: "brainstorm",
    version: 1,
    name: "Brainstorm",
    description: "Generating ideas: the problem, every idea raised, and which ones to pursue.",
    meeting_context: "An open-ended session to generate options. Preserve the breadth of ideas, group related ones, and be clear about which ideas the group chose to take forward.",
    sections: &[
        section(
            "Problem",
            Paragraph,
            "The question or problem being explored, and any constraints given.",
        ),
        section(
            "Ideas",
            Bullets,
            "Every distinct idea, grouped by theme, with who suggested it. Keep the unusual ones; do not judge them.",
        ),
        section(
            "Favourites",
            Bullets,
            "Ideas the group got excited about or voted for, and why.",
        ),
        section(
            "Next steps",
            Todo,
            "Ideas to explore further or test, with the owner.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "brainstorm",
        r"\b(brainstorm\w*|ideation|jam session)\b",
    )],
};

/// A company or org-wide meeting.
pub(super) const ALL_HANDS: BuiltinTemplate = BuiltinTemplate {
    id: "all_hands",
    version: 1,
    name: "All-hands",
    description: "A company or department meeting: announcements, results, team updates and Q&A.",
    meeting_context: "Leadership updating a whole company or department. Capture announcements, dates and numbers exactly as stated, and every question asked with the answer given.",
    sections: &[
        section(
            "Announcements",
            Bullets,
            "Each announcement, with dates and numbers exactly as stated.",
        ),
        section(
            "Business update",
            Bullets,
            "Results, metrics and progress against goals.",
        ),
        section(
            "Team updates",
            Bullets,
            "Updates from each team or presenter, attributed.",
        ),
        section(
            "Q&A",
            Bullets,
            "Each question in bold, followed by the answer and who gave it.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "all-hands",
        r"\b(all[\s-]hands|town ?hall|company meeting)\b",
    )],
};
