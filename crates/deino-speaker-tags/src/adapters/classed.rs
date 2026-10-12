//! The scanner for Chromium web content, where an element's CSS classes are
//! visible (`AXDOMClassList`): a participant's tile holds a name label and a
//! voice indicator, recognised by their classes.
//!
//! Tiles have no class of their own that survives Meet's releases, so a lit
//! indicator belongs to the **nearest enclosing element that holds exactly
//! one name**: its tile. An indicator whose nearest named ancestor holds
//! several names (a grid without tiles in between) is ambiguous and ignored.

use crate::label::Label;
use crate::reading::Scan;
use crate::tree::AxNode;

/// One app's classes.
#[derive(Clone, Copy, Debug)]
pub(super) struct Classes {
    /// Classes of a participant's name label.
    pub(super) name: &'static [&'static str],
    /// Classes of a voice indicator, lit or not.
    pub(super) indicator: &'static [&'static str],
    /// Classes of a lit voice indicator.
    pub(super) speaking: &'static [&'static str],
}

/// What a subtree holds, passed up to its parent.
#[derive(Default)]
struct Subtree {
    /// The distinct names labelled in it.
    names: Vec<Label>,
    /// A lit indicator in it not yet given to a tile.
    pending_lit: bool,
}

/// Every name label and indicator under `window`.
pub(super) fn scan(window: &AxNode, classes: &Classes) -> Scan {
    let mut scan = Scan::default();
    visit(window, classes, &mut scan);
    scan
}

fn visit(node: &AxNode, classes: &Classes, scan: &mut Scan) -> Subtree {
    let mut subtree = Subtree::default();
    if let Some(label) = name_label(node, classes) {
        scan.labels.push(label.clone());
        subtree.names.push(label);
    }
    if node.has_class(classes.indicator) || node.has_class(classes.speaking) {
        scan.indicators += 1;
    }
    subtree.pending_lit = node.has_class(classes.speaking);
    for child in &node.children {
        let below = visit(child, classes, scan);
        subtree.pending_lit |= below.pending_lit;
        for name in below.names {
            if !subtree.names.iter().any(|known| known.name == name.name) {
                subtree.names.push(name);
            }
        }
    }
    settle(&mut subtree, scan);
    subtree
}

/// Gives a pending lit indicator to this element when it is a tile (one
/// name), or drops it when the element holds several names.
fn settle(subtree: &mut Subtree, scan: &mut Scan) {
    if !subtree.pending_lit {
        return;
    }
    match subtree.names.as_slice() {
        [] => {}
        [tile] => {
            scan.lit.push(tile.clone());
            subtree.pending_lit = false;
        }
        _ => subtree.pending_lit = false,
    }
}

/// The node's name, if it is a name label: its own text, or its first text
/// child's (Chromium may put a label's text on a static text child).
fn name_label(node: &AxNode, classes: &Classes) -> Option<Label> {
    if !node.has_class(classes.name) {
        return None;
    }
    let text = node
        .text()
        .or_else(|| node.descendants().skip(1).find_map(AxNode::text))?;
    Label::parse(text)
}
