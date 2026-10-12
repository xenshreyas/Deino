//! `vision.describe` (CAP-09): a shared-screen snapshot described in words
//! for the notes. The picture goes with the user message as an image part;
//! nothing else about the meeting is sent.

use askama::Template;

/// The description's rules.
#[derive(Template)]
#[template(path = "vision_describe.system.md.jinja", escape = "none")]
pub struct VisionDescribeSystem;

/// The request that goes with the picture.
#[derive(Template)]
#[template(path = "vision_describe.user.md.jinja", escape = "none")]
pub struct VisionDescribeUser;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::PromptText;

    #[test]
    fn the_rules_keep_to_what_is_visible() {
        insta::assert_snapshot!(VisionDescribeSystem.text(), @r"
        # General Instructions

        - You describe a snapshot of a screen shared in a meeting, so the meeting's notes can mention what was shown.
        - Say what kind of content it is (a slide, a document, a chart, a design, code, a web page) and what it shows: its title, the main points, and the trend or numbers of any chart.
        - Use only what is visible. Never guess names, figures or text you cannot read.
        - Leave out window chrome, toolbars and the video tiles of participants.

        # Format Requirements

        - Plain text in English, at most five short sentences, no Markdown.
        ");
        insta::assert_snapshot!(VisionDescribeUser.text(), @"Describe this shared screen.");
    }
}
