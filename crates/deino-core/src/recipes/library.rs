//! [`LIBRARY`]: the built-in recipes (PARITY_PLAN AI-04's twenty).
//!
//! Each prompt asks for something one meeting can answer. The three that
//! need several meetings ("Weekly recap", "How have I changed", "Prep my
//! day") are [`RecipeKind::Multi`]: listed as WIP until cross-meeting chat
//! (AI-02) returns (D13). "How I like to work" and "Prep me for this person"
//! are adapted to read one meeting.

use super::RecipeKind;

/// A recipe the app ships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuiltinRecipe {
    /// Its `/shortcut`.
    pub slug: &'static str,
    /// Its name.
    pub name: &'static str,
    /// One line on what it does.
    pub description: &'static str,
    /// What it asks the model.
    pub prompt: &'static str,
    /// What it runs on.
    pub kind: RecipeKind,
}

const fn one(
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    prompt: &'static str,
) -> BuiltinRecipe {
    BuiltinRecipe {
        slug,
        name,
        description,
        prompt,
        kind: RecipeKind::Single,
    }
}

const fn many(
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    prompt: &'static str,
) -> BuiltinRecipe {
    BuiltinRecipe {
        slug,
        name,
        description,
        prompt,
        kind: RecipeKind::Multi,
    }
}

/// Every built-in recipe, in the Recipes page's order.
pub const LIBRARY: [BuiltinRecipe; 20] = [
    one(
        "coach-me",
        "Coach me",
        "Feedback on how you showed up, tied to moments in the meeting.",
        "Coach me on how I showed up in this meeting. Name two things I did well and two or three specific things to do differently next time, each tied to a moment in the transcript.",
    ),
    one(
        "prep-me-for-this-person",
        "Prep me for this person",
        "What to remember before you next talk to the people you met.",
        "Prepare me for my next conversation with the other people in this meeting: who they are, what they care about, what they asked for, and what I promised them.",
    ),
    one(
        "write-a-brief",
        "Write a brief",
        "A short brief for someone who missed the meeting.",
        "Write a short brief of this meeting for someone who missed it: why we met, what was decided, the open questions, and the next steps with owners.",
    ),
    one(
        "write-follow-up-email",
        "Write follow-up email",
        "A follow-up email to the attendees, in your voice.",
        "Draft a follow-up email to the other attendees, in my voice: thank them, recap the decisions and the next steps with owners and dates, and list the open questions. Keep it under 200 words and start with a subject line.",
    ),
    one(
        "summarize-for-my-manager",
        "Summarize for my manager",
        "Outcomes, risks and asks, the way a manager reads them.",
        "Summarize this meeting for my manager in five bullets or fewer: the outcome, any decision they should know about, risks, and anything I need from them.",
    ),
    one(
        "extract-decisions",
        "Extract decisions",
        "Every decision, who made it and why.",
        "List every decision made in this meeting, with who made it and the reason given. Leave out ideas that were discussed but not decided.",
    ),
    one(
        "list-open-questions",
        "List open questions",
        "What is still unanswered, and who could answer it.",
        "List the questions this meeting left open, and for each one who is best placed to answer it.",
    ),
    one(
        "customer-pain-points",
        "Customer pain points",
        "The problems the customer described, in their words.",
        "List the pain points the customer described, each with a short quote in their own words and how much it seems to cost them.",
    ),
    one(
        "feature-requests",
        "Feature requests",
        "What was asked for, and why it matters to them.",
        "List the features or changes that were requested, who asked, the problem behind each request, and how urgent it sounded.",
    ),
    one(
        "competitor-mentions",
        "Competitor mentions",
        "Every competitor named, and what was said about them.",
        "List every competitor or alternative product mentioned, what was said about it, and whether it sounded like a threat.",
    ),
    one(
        "objections-and-responses",
        "Objections & responses",
        "Each objection raised and how it was answered.",
        "List each objection or concern that was raised, how it was answered in the meeting, and a stronger answer for next time where the answer was weak.",
    ),
    one(
        "hiring-scorecard",
        "Hiring scorecard",
        "A candidate scorecard with evidence for each rating.",
        "Write a hiring scorecard for the candidate: for each skill that came up, a rating from 1 to 4 with the evidence behind it, then strengths, concerns and a hire or no-hire recommendation.",
    ),
    one(
        "prd-draft",
        "PRD draft from discussion",
        "A product requirements draft from what was discussed.",
        "Draft a short product requirements document from this discussion: the problem, the users, goals and non-goals, requirements, open questions and risks. Use only what was said.",
    ),
    many(
        "weekly-recap",
        "Weekly recap",
        "A recap of your week across meetings.",
        "Recap my week across these meetings: the main outcomes, decisions, open threads and what needs my attention next week.",
    ),
    many(
        "how-have-i-changed",
        "How have I changed",
        "How your focus and habits shifted over time.",
        "Compare my earlier and later meetings and describe how my focus, priorities and habits have changed.",
    ),
    one(
        "how-i-like-to-work",
        "How I like to work",
        "Your working style, as this meeting shows it.",
        "From how I ran and took part in this meeting, describe how I like to work: how I make decisions, communicate and follow up. Give examples.",
    ),
    many(
        "prep-my-day",
        "Prep my day",
        "Get ready for today's meetings.",
        "Prepare me for today's meetings: for each one, who I am meeting, what happened last time and what to follow up on.",
    ),
    one(
        "mochary-1-1-review",
        "Mochary-style 1:1 review",
        "Review a 1:1 the Mochary way: issues, owners, commitments.",
        "Review this 1:1 in the style of the Mochary Method: the issues raised, whether each was solved or has an owner and a next step, the commitments each person made, and any feedback that was not said directly.",
    ),
    one(
        "risks-and-blockers",
        "Risks & blockers",
        "Risks and blockers, with owners and mitigations.",
        "List the risks and blockers raised in this meeting, how serious each is, who owns it and any mitigation that was agreed.",
    ),
    one(
        "what-did-i-commit-to",
        "What did I commit to?",
        "Everything you promised, with dates.",
        "List everything I committed to in this meeting, to whom, and by when.",
    ),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::recipes::{MAX_DESCRIPTION_CHARS, is_valid_slug};

    #[test]
    fn every_built_in_has_a_unique_valid_slug_and_a_prompt() {
        let slugs: BTreeSet<&str> = LIBRARY.iter().map(|recipe| recipe.slug).collect();

        assert_eq!(slugs.len(), LIBRARY.len());
        for recipe in LIBRARY {
            assert!(is_valid_slug(recipe.slug), "{}", recipe.slug);
            assert!(!recipe.prompt.trim().is_empty() && !recipe.name.is_empty());
            assert!(recipe.description.len() <= MAX_DESCRIPTION_CHARS);
        }
    }

    #[test]
    fn only_the_cross_meeting_three_are_multi() {
        let multi: Vec<&str> = LIBRARY
            .iter()
            .filter(|recipe| recipe.kind == RecipeKind::Multi)
            .map(|recipe| recipe.slug)
            .collect();

        assert_eq!(multi, ["weekly-recap", "how-have-i-changed", "prep-my-day"]);
    }
}
