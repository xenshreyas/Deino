//! The notepad's checklist shorthand, read from Markdown.
//!
//! GFM only knows `- [ ] item`. People (and the notepad's own input rule)
//! also write `[] item`, `[ ] item` and `- [] item`, and notes typed or
//! pasted that way used to come back as literal brackets. Before parsing,
//! each such line outside a code fence is rewritten to the GFM form, so it
//! becomes a task item. Escaped brackets (`\[\] item`, which
//! [`to_markdown`](crate::to_markdown) writes for literal text) never match,
//! so round trips are unchanged.

use std::borrow::Cow;

/// `markdown` with every checklist shorthand line in GFM form.
pub(super) fn normalize_task_markers(markdown: &str) -> Cow<'_, str> {
    if !markdown.contains('[') {
        return Cow::Borrowed(markdown);
    }
    let mut fence: Option<&str> = None;
    let mut changed = false;
    let lines: Vec<Cow<'_, str>> = markdown
        .split_inclusive('\n')
        .map(|line| {
            fence = next_fence(fence, line);
            if fence.is_some() {
                return Cow::Borrowed(line);
            }
            match task_line(line) {
                Some(rewritten) => {
                    changed = true;
                    Cow::Owned(rewritten)
                }
                None => Cow::Borrowed(line),
            }
        })
        .collect();
    if changed {
        Cow::Owned(lines.concat())
    } else {
        Cow::Borrowed(markdown)
    }
}

/// The fence still open after `line`: a line starting with three or more
/// backticks or tildes opens a fence, and the same marker closes it.
fn next_fence<'a>(open: Option<&'a str>, line: &'a str) -> Option<&'a str> {
    let trimmed = line.trim_start();
    let marker = ["```", "~~~"]
        .into_iter()
        .find(|marker| trimmed.starts_with(marker));
    match (open, marker) {
        (Some(current), Some(marker)) if current == marker => None,
        (None, Some(marker)) => Some(marker),
        (open, _) => open,
    }
}

/// `[] item`, `[ ] item`, `[x] item` (optionally after a `-`, `*` or `+`
/// bullet, which is kept so neighbouring lists stay apart) as GFM, keeping
/// the indentation. Lines GFM already reads as tasks are left alone.
fn task_line(line: &str) -> Option<String> {
    let body = line.trim_start();
    let indent = line.get(..line.len() - body.len()).unwrap_or_default();
    let bullet = ['-', '*', '+'].into_iter().find(|bullet| {
        body.strip_prefix(*bullet)
            .is_some_and(|rest| rest.starts_with(' '))
    });
    let unbulleted = bullet.map_or(body, |_| body.get(1..).unwrap_or_default().trim_start());
    let (marker, checked) = [("[]", false), ("[ ]", false), ("[x]", true), ("[X]", true)]
        .into_iter()
        .find(|(marker, _)| unbulleted.starts_with(marker))?;
    let text = unbulleted.get(marker.len()..)?.strip_prefix([' ', '\t'])?;
    if text.trim().is_empty() || (bullet.is_some() && marker != "[]") {
        return None;
    }
    let mark = if checked { 'x' } else { ' ' };
    let bullet = bullet.unwrap_or('-');
    Some(format!("{indent}{bullet} [{mark}] {}", text.trim_start()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checklist_shorthand_becomes_gfm() {
        assert_eq!(
            normalize_task_markers("[] send the deck\n[x] done\n- [] bulleted\n  [ ] nested\n"),
            "- [ ] send the deck\n- [x] done\n- [ ] bulleted\n  - [ ] nested\n"
        );
    }

    #[test]
    fn escaped_brackets_code_and_links_are_left_alone() {
        for markdown in [
            "\\[\\] literal",
            "```\n[] in code\n```\n",
            "[link](https://deino.dev) text",
            "[]",
            "[]text",
            "- [ ] already gfm",
        ] {
            assert_eq!(normalize_task_markers(markdown), markdown, "{markdown}");
        }
    }
}
