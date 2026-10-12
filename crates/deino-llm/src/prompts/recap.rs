//! `recap` (ORG-09): the year in review, written from aggregated numbers
//! and meeting titles only. `deino_core::stats` renders every input to
//! text; these structs only place it.

use askama::Template;

/// `recap`: the rules of the year in review.
#[derive(Template)]
#[template(path = "recap.system.md.jinja", escape = "none")]
pub struct RecapSystem;

/// `recap`: one year's numbers and titles.
#[derive(Template)]
#[template(path = "recap.user.md.jinja", escape = "none")]
pub struct RecapUser<'a> {
    /// The calendar year, for example `2026`.
    pub year: &'a str,
    /// `Label: value` lines: meetings, hours, people, busiest days.
    pub numbers: &'a [String],
    /// `YYYY-MM-DD Title` lines, oldest first.
    pub titles: &'a [String],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::PromptText;

    #[test]
    fn the_user_prompt_places_numbers_and_titles() {
        let numbers = ["Meetings: 3".to_owned(), "Hours recorded: 2".to_owned()];
        let titles = ["2026-01-05 Kickoff".to_owned()];

        let text = RecapUser {
            year: "2026",
            numbers: &numbers,
            titles: &titles,
        }
        .text();

        assert_eq!(
            text,
            "<numbers>\nYear: 2026\nMeetings: 3\nHours recorded: 2\n</numbers>\n\n\
             <titles>\n- 2026-01-05 Kickoff\n</titles>\n\n\
             Now write the 2026 year in review."
        );
    }

    #[test]
    fn the_system_prompt_forbids_inventing_content() {
        let text = RecapSystem.text();

        assert!(text.contains("never invent what was said"));
        assert!(text.contains("under 250 words"));
    }
}
