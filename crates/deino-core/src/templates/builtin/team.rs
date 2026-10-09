//! Built-ins for a team's own rhythm: general notes, 1:1s, stand-ups,
//! syncs, kick-offs, planning and retrospectives.

use deino_domain::SectionFormat::{Bullets, Paragraph, Table, Todo};

use super::{BuiltinRule, BuiltinTemplate, GENERAL_TEMPLATE_ID, section};

/// The default: adapts to any meeting (Granola's "Auto").
pub(super) const GENERAL: BuiltinTemplate = BuiltinTemplate {
    id: GENERAL_TEMPLATE_ID,
    version: 1,
    name: "General",
    description: "Adapts to any meeting: an overview, the discussion by topic, decisions and next steps.",
    meeting_context: "Use when no more specific template fits. Infer the meeting's purpose from the title, attendees and conversation, and let the topics actually discussed shape the notes.",
    sections: &[
        section(
            "Overview",
            Bullets,
            "Three to five bullets: why the meeting happened, what was concluded and what is still open.",
        ),
        section(
            "Discussion",
            Bullets,
            "Group the conversation by topic. Lead each group with the topic in bold, then the key points, figures and who said what. Leave out small talk and logistics.",
        ),
        section(
            "Decisions",
            Bullets,
            "Each decision with its reason when one was given. Write \"None\" if nothing was decided.",
        ),
        section(
            "Next steps",
            Todo,
            "Every commitment someone made, with the owner and the due date when stated. Only what someone agreed to do.",
        ),
    ],
    rules: &[],
};

/// A recurring one-on-one.
pub(super) const ONE_ON_ONE: BuiltinTemplate = BuiltinTemplate {
    id: "one_on_one",
    version: 1,
    name: "1:1",
    description: "A recurring one-on-one between a manager and a report, or two peers.",
    meeting_context: "Two people catching up regularly. Capture what each person raised, feedback in both directions and agreed follow-ups. Keep the tone personal and candid, and do not editorialise about anyone's performance.",
    sections: &[
        section(
            "Check-in",
            Paragraph,
            "Two or three sentences on how each person is doing (workload, energy, morale), only as far as they shared it.",
        ),
        section(
            "Topics",
            Bullets,
            "Each topic either person raised, with the key points and where it landed.",
        ),
        section(
            "Feedback",
            Bullets,
            "Feedback given in either direction, attributed, in the giver's own words where possible. Omit the section if none was given.",
        ),
        section(
            "Growth",
            Bullets,
            "Goals, skills, opportunities or concerns about career growth. Omit the section if not discussed.",
        ),
        section(
            "Action items",
            Todo,
            "Follow-ups for either person, with the owner and due date when stated.",
        ),
    ],
    rules: &[
        BuiltinRule::titled(
            "1:1 in the title",
            r"\b1\s?[:\-/]\s?1\b|\bone[\s-]on[\s-]one\b|\b1\s?on\s?1\b",
        ),
        BuiltinRule::untitled("two colleagues")
            .internal()
            .attendees(2, 2),
    ],
};

/// A daily stand-up.
pub(super) const STANDUP: BuiltinTemplate = BuiltinTemplate {
    id: "standup",
    version: 1,
    name: "Stand-up",
    description: "A short daily stand-up: progress, plans and blockers per person.",
    meeting_context: "A time-boxed team check-in. Organise by person, keep each update to its essentials and surface blockers prominently.",
    sections: &[
        section(
            "Updates",
            Table,
            "One row per person: their name, what they finished since the last stand-up, and what they are doing next. Keep each cell to a short phrase.",
        ),
        section(
            "Blockers",
            Bullets,
            "Each blocker, who is blocked and who offered to help. Write \"None\" if nobody is blocked.",
        ),
        section(
            "Follow-ups",
            Todo,
            "Discussions taken offline and anything someone agreed to do, with the owner.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "stand-up",
        r"\bstand[\s-]?up\b|\bdaily (sync|scrum|check[\s-]?in)\b|\bscrum\b",
    )],
};

/// A recurring team meeting.
pub(super) const WEEKLY_TEAM_SYNC: BuiltinTemplate = BuiltinTemplate {
    id: "weekly_team_sync",
    version: 1,
    name: "Weekly team sync",
    description: "A regular team meeting: status against priorities, risks and decisions.",
    meeting_context: "A recurring team meeting about progress against priorities. Emphasise what changed since last time, risks and decisions, and leave out routine status that did not change.",
    sections: &[
        section(
            "Highlights",
            Bullets,
            "The three to five most important updates since the last sync.",
        ),
        section(
            "Status by project",
            Table,
            "One row per project or workstream: name, owner, status (on track, at risk or off track) and the key update.",
        ),
        section(
            "Risks and blockers",
            Bullets,
            "Each risk or blocker with its impact, owner and mitigation, if discussed.",
        ),
        section(
            "Decisions",
            Bullets,
            "Decisions made, with the reason when given.",
        ),
        section(
            "Action items",
            Todo,
            "Every commitment, with the owner and due date when stated.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "team sync",
        r"\b(weekly|team sync|team meeting|status (update|meeting)|project sync)\b",
    )
    .internal()],
};

/// The first meeting of a project.
pub(super) const PROJECT_KICKOFF: BuiltinTemplate = BuiltinTemplate {
    id: "project_kickoff",
    version: 1,
    name: "Project kick-off",
    description: "The start of a project: goals, scope, roles, plan and risks.",
    meeting_context: "The first meeting of a new project. Capture the shared understanding the team leaves with: why the project exists, what is in and out of scope, who owns what and how success will be measured.",
    sections: &[
        section(
            "Goals and success criteria",
            Bullets,
            "Why the project exists and how the team will know it succeeded, with any metrics or targets named.",
        ),
        section(
            "Scope",
            Bullets,
            "What is in scope and, separately, what was explicitly ruled out.",
        ),
        section(
            "Roles",
            Table,
            "One row per person: their name and what they own.",
        ),
        section(
            "Milestones",
            Table,
            "Milestone, target date and owner, only for dates actually discussed.",
        ),
        section(
            "Risks and open questions",
            Bullets,
            "Each risk, dependency or unanswered question, with an owner when one was named.",
        ),
        section(
            "Next steps",
            Todo,
            "Every commitment, with the owner and due date when stated.",
        ),
    ],
    rules: &[BuiltinRule::titled("kick-off", r"\bkick[\s-]?off\b")],
};

/// Planning the next sprint.
pub(super) const SPRINT_PLANNING: BuiltinTemplate = BuiltinTemplate {
    id: "sprint_planning",
    version: 1,
    name: "Sprint planning",
    description: "Planning the next sprint: the goal, committed work, capacity and risks.",
    meeting_context: "An agile team planning its next iteration. Record the sprint goal and exactly what the team committed to, and note what was deferred and why.",
    sections: &[
        section(
            "Sprint goal",
            Paragraph,
            "The goal in one or two sentences, as the team phrased it.",
        ),
        section(
            "Committed work",
            Table,
            "Item or ticket, owner and estimate when given. Only items the team committed to.",
        ),
        section(
            "Capacity",
            Bullets,
            "Availability, time off and other constraints that affect the plan.",
        ),
        section(
            "Deferred",
            Bullets,
            "Items discussed but left out of the sprint, with the reason.",
        ),
        section(
            "Risks and dependencies",
            Bullets,
            "Anything that could stop the team meeting the goal, with an owner when named.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "sprint planning",
        r"\b(sprint|iteration) planning\b|\bsprint plan\b",
    )],
};

/// Looking back on a period of work.
pub(super) const RETROSPECTIVE: BuiltinTemplate = BuiltinTemplate {
    id: "retrospective",
    version: 1,
    name: "Retrospective",
    description: "Looking back on a sprint or project: what went well, what did not and what to change.",
    meeting_context: "A blameless review of a recent period of work. Attribute observations neutrally, cluster similar points and make the agreed experiments concrete.",
    sections: &[
        section(
            "What went well",
            Bullets,
            "Points the team wants to keep, clustered by theme.",
        ),
        section(
            "What did not go well",
            Bullets,
            "Problems and frustrations, clustered by theme, without assigning blame.",
        ),
        section(
            "Ideas and experiments",
            Bullets,
            "Changes proposed, noting which ones the team voted for or agreed on.",
        ),
        section(
            "Action items",
            Todo,
            "Each agreed experiment with its owner and how the team will know it worked.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "retrospective",
        r"\b(retro|retrospective|post[\s-]?mortem)\b",
    )],
};
