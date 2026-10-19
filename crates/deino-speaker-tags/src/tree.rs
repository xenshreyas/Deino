//! [`AxNode`]: a snapshot of part of an app's Accessibility tree, the one
//! shape the adapters read, whether it came from the live app (`macos/`) or
//! from a fixture dump (`fixtures/`).
//!
//! Only the attributes the adapters use are kept, under their short names
//! (`AXRole` is `role`, `AXDescription` is `description`, Chromium's
//! `AXDOMClassList` is `domClasses`, a window's `AXDocument` is `document`,
//! `AXPosition` and `AXSize` together are `frame`). Absent attributes are
//! left out of the JSON, so dumps stay readable, and dumps made before an
//! attribute was kept still read.

use serde::{Deserialize, Serialize};

/// One element of an Accessibility tree and the elements under it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AxNode {
    /// `AXRole`, for example `AXWindow`, `AXGroup`, `AXStaticText`.
    pub role: String,
    /// `AXSubrole`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,
    /// `AXTitle`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `AXDescription`: web content's `aria-label`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `AXValue`, when it is text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// `AXIdentifier`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// `AXDOMClassList`: the element's CSS classes, in Chromium web content.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dom_classes: Vec<String>,
    /// `AXDocument`: a browser window's page address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    /// `AXPosition` and `AXSize`: where the element is on screen. Read for
    /// the roles an adapter compares by size (Zoom's video tiles).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    /// The element's children, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AxNode>,
}

/// Where an element is, in points: `AXPosition` (screen coordinates, the
/// top-left corner) and `AXSize`. Adapters compare sizes, never positions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The left edge.
    pub x: i32,
    /// The top edge.
    pub y: i32,
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
}

impl Frame {
    /// The area, in square points.
    pub fn area(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// A window as seen before its contents are read: enough to tell a meeting
/// window from the others cheaply.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowSummary {
    /// The window's title.
    pub title: Option<String>,
    /// The page address, for a browser window.
    pub document: Option<String>,
}

impl AxNode {
    /// The node and every node under it, depth first.
    pub fn descendants(&self) -> Descendants<'_> {
        Descendants { stack: vec![self] }
    }

    /// The node's text as an app labels it: the title, else the
    /// description, else the value; blank text counts as none.
    pub fn text(&self) -> Option<&str> {
        [&self.title, &self.description, &self.value]
            .into_iter()
            .filter_map(Option::as_deref)
            .map(str::trim)
            .find(|text| !text.is_empty())
    }

    /// Whether the node has the role `role`.
    pub fn is(&self, role: &str) -> bool {
        self.role == role
    }

    /// Whether the node carries any of `classes` (Chromium web content).
    pub fn has_class(&self, classes: &[&str]) -> bool {
        self.dom_classes
            .iter()
            .any(|class| classes.contains(&class.as_str()))
    }

    /// Whether a button under the node is labelled `label`, ignoring case
    /// and a shortcut hint after it ("Leave (⌘⇧H)").
    pub fn has_button(&self, label: &str) -> bool {
        self.descendants()
            .filter(|node| node.is(BUTTON_ROLE))
            .filter_map(AxNode::text)
            .any(|text| {
                let main = text.split_once(" (").map_or(text, |(main, _)| main);
                main.eq_ignore_ascii_case(label)
            })
    }

    /// A window's summary (its title and page address).
    pub fn summary(&self) -> WindowSummary {
        WindowSummary {
            title: self.title.clone(),
            document: self.document.clone(),
        }
    }
}

/// The Accessibility role of a button.
pub const BUTTON_ROLE: &str = "AXButton";

/// The Accessibility role of a window.
pub const WINDOW_ROLE: &str = "AXWindow";

/// Depth-first iteration over a tree ([`AxNode::descendants`]).
#[derive(Debug)]
pub struct Descendants<'a> {
    stack: Vec<&'a AxNode>,
}

impl<'a> Iterator for Descendants<'a> {
    type Item = &'a AxNode;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        self.stack.extend(node.children.iter().rev());
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: &str, title: Option<&str>, children: Vec<AxNode>) -> AxNode {
        AxNode {
            role: role.into(),
            title: title.map(Into::into),
            children,
            ..AxNode::default()
        }
    }

    #[test]
    fn descendants_walk_depth_first_in_order() {
        let tree = node(
            "AXWindow",
            Some("w"),
            vec![
                node(
                    "AXGroup",
                    Some("a"),
                    vec![node("AXButton", Some("b"), vec![])],
                ),
                node("AXGroup", Some("c"), vec![]),
            ],
        );

        let titles: Vec<&str> = tree.descendants().filter_map(AxNode::text).collect();

        assert_eq!(titles, ["w", "a", "b", "c"]);
        assert!(tree.has_button("B"));
        assert!(!tree.has_button("a"));
    }

    #[test]
    fn text_prefers_the_title_and_skips_blanks() {
        let labelled = AxNode {
            title: Some("  ".into()),
            description: Some("Alex Chen, muted".into()),
            value: Some("ignored".into()),
            ..AxNode::default()
        };

        assert_eq!(labelled.text(), Some("Alex Chen, muted"));
        assert_eq!(AxNode::default().text(), None);
    }

    #[test]
    fn a_frame_reads_back_and_dumps_without_one_still_read() {
        let tile = AxNode {
            role: "AXTabGroup".into(),
            frame: Some(Frame {
                x: 0,
                y: 93,
                width: 1_261,
                height: 709,
            }),
            ..AxNode::default()
        };

        let json = serde_json::to_string(&tile).unwrap();

        assert_eq!(
            json,
            r#"{"role":"AXTabGroup","frame":{"x":0,"y":93,"width":1261,"height":709}}"#
        );
        assert_eq!(serde_json::from_str::<AxNode>(&json).unwrap(), tile);
        assert_eq!(tile.frame.map(|frame| frame.area()), Some(894_049));
        let old: AxNode = serde_json::from_str(r#"{"role":"AXGroup"}"#).unwrap();
        assert_eq!(old.frame, None);
    }

    #[test]
    fn dumps_leave_out_absent_attributes_and_read_back() {
        let tree = AxNode {
            role: "AXGroup".into(),
            dom_classes: vec!["notranslate".into()],
            ..AxNode::default()
        };

        let json = serde_json::to_string(&tree).unwrap();

        assert_eq!(json, r#"{"role":"AXGroup","domClasses":["notranslate"]}"#);
        assert_eq!(serde_json::from_str::<AxNode>(&json).unwrap(), tree);
    }
}
