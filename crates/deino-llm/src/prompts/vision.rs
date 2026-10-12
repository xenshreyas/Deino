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
        insta::assert_snapshot!(VisionDescribeSystem.text(), @"");
        insta::assert_snapshot!(VisionDescribeUser.text(), @"");
    }
}
