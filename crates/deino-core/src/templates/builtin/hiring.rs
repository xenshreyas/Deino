//! Built-ins for conversations about people: research interviews, hiring
//! and coaching.

use deino_domain::SectionFormat::{Bullets, Paragraph, Table, Todo};

use super::{BuiltinRule, BuiltinTemplate, section};

/// Research with a user.
pub(super) const USER_INTERVIEW: BuiltinTemplate = BuiltinTemplate {
    id: "user_interview",
    version: 1,
    name: "User interview",
    description: "Research with a user or customer: their context, workflow, pains and quotes.",
    meeting_context: "A research conversation to learn from a user, not to sell to them. Keep observed facts and direct quotes separate from interpretation, and do not overstate how widely a point applies.",
    sections: &[
        section(
            "Participant",
            Bullets,
            "Who they are: role, type of company, experience, and how they use the relevant tools.",
        ),
        section(
            "Context and workflow",
            Bullets,
            "How they do the job being studied today, step by step.",
        ),
        section(
            "Pain points",
            Bullets,
            "Problems and frustrations, with how often they happen and what they cost.",
        ),
        section(
            "Quotes",
            Bullets,
            "Verbatim quotes worth keeping, each in quotation marks.",
        ),
        section(
            "Insights",
            Bullets,
            "Patterns and opportunities the interview suggests, labelled as interpretation.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "research interview",
        r"\b(user|customer|research|discovery) (interview|research|session)\b|\busability\b",
    )
    .external()],
};

/// Interviewing a candidate.
pub(super) const CANDIDATE_INTERVIEW: BuiltinTemplate = BuiltinTemplate {
    id: "candidate_interview",
    version: 1,
    name: "Candidate interview",
    description: "Interviewing a job candidate: background, answers, strengths, concerns and a recommendation.",
    meeting_context: "The user is interviewing a candidate. Record evidence the hiring team can assess: what the candidate said and did, not impressions of their personality, and nothing about protected characteristics.",
    sections: &[
        section(
            "Candidate and role",
            Bullets,
            "The role, the interview stage and the candidate's background as discussed.",
        ),
        section(
            "Questions and answers",
            Bullets,
            "Each question asked, in bold, followed by a summary of the answer and the specific examples given.",
        ),
        section(
            "Strengths",
            Bullets,
            "Evidence of strong skills or experience, each tied to a specific answer.",
        ),
        section(
            "Concerns",
            Bullets,
            "Gaps or concerns, each tied to a specific answer.",
        ),
        section(
            "Candidate's questions",
            Bullets,
            "Questions the candidate asked, with the answers given.",
        ),
        section(
            "Recommendation",
            Paragraph,
            "The interviewer's stated hire or no-hire leaning and their reasons. Write \"No recommendation stated\" if they gave none.",
        ),
    ],
    rules: &[
        BuiltinRule::titled("candidate interview", r"\binterview\b").external(),
        BuiltinRule::titled(
            "phone screen",
            r"\b(phone|technical|tech|recruiter) screen\b",
        ),
    ],
};

/// The hiring team's decision meeting.
pub(super) const INTERVIEW_DEBRIEF: BuiltinTemplate = BuiltinTemplate {
    id: "interview_debrief",
    version: 1,
    name: "Interview debrief",
    description: "The hiring team comparing evidence on a candidate and reaching a decision.",
    meeting_context: "Interviewers sharing evidence and deciding on a candidate. Attribute each interviewer's assessment, keep feedback evidence-based, and record the final decision and next steps clearly.",
    sections: &[
        section(
            "Candidate and role",
            Bullets,
            "The candidate, the role and the interviews completed.",
        ),
        section(
            "Feedback by interviewer",
            Table,
            "Interviewer, focus area, assessment (strong yes to strong no) and the key evidence.",
        ),
        section(
            "Strengths",
            Bullets,
            "Strengths the panel agreed on, with the evidence.",
        ),
        section(
            "Concerns",
            Bullets,
            "Concerns raised, by whom, and whether they were resolved.",
        ),
        section(
            "Decision",
            Paragraph,
            "The decision (hire, no hire or more interviews) and the reasoning. Write \"Undecided\" if none was reached.",
        ),
        section(
            "Next steps",
            Todo,
            "References, offer details or further interviews, with the owner.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "hiring debrief",
        r"\b(debrief|hiring (committee|decision|sync))\b",
    )
    .internal()],
};

/// Coaching or mentoring.
pub(super) const COACHING_SESSION: BuiltinTemplate = BuiltinTemplate {
    id: "coaching_session",
    version: 1,
    name: "Coaching session",
    description: "A coaching or mentoring session: the focus, insights, advice and commitments.",
    meeting_context: "A coach or mentor working with someone on their development. Capture the coachee's goals and reflections in their own words, and the commitments they made. Keep the tone supportive; these notes are private.",
    sections: &[
        section(
            "Focus",
            Paragraph,
            "What the session focused on and why it matters to the coachee now.",
        ),
        section(
            "Reflections and insights",
            Bullets,
            "Realisations, reframes and perspectives that came up, attributed.",
        ),
        section(
            "Advice and frameworks",
            Bullets,
            "Advice, tools or frameworks offered, and how they apply.",
        ),
        section(
            "Commitments",
            Todo,
            "What the coachee committed to try before the next session.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "coaching",
        r"\b(coaching|mentoring|mentor(ship)?)\b",
    )],
};
