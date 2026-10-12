//! [`AxWindows`]: an app's call windows read into [`AxNode`] trees.
//!
//! Every attribute read is a round trip to the other app, so a read is
//! bounded: a short messaging timeout per request (a hung app cannot stall
//! the watcher), a node budget and a depth limit, and subtrees that never
//! hold participants (menus, scroll bars, text fields such as a chat
//! composer) are skipped. Only the attributes the adapters use are read
//! live; the capture tool reads them all. Nothing read here is logged.

use cidre::{ax, cf};
use deino_domain::CallApp;

use super::dom;
use crate::error::AxError;
use crate::ports::{Pid, WindowReader};
use crate::tree::{AxNode, WINDOW_ROLE, WindowSummary};

/// How long one Accessibility request may take, in seconds.
const AX_TIMEOUT_SECS: f32 = 0.1;

/// At most this many windows are looked at per app.
const MAX_WINDOWS: usize = 8;

/// At most this many elements are read per window.
const MAX_NODES: usize = 3_000;

/// Elements deeper than this are not read.
const MAX_DEPTH: usize = 48;

/// Roles whose subtrees never hold a participant.
const SKIPPED_ROLES: &[&str] = &[
    "AXMenuBar",
    "AXMenu",
    "AXScrollBar",
    "AXTextArea",
    "AXTextField",
    "AXImage",
];

/// Web content, where Chromium exposes CSS classes.
const WEB_AREA_ROLE: &str = "AXWebArea";

/// Chromium's switch for exposing web content without a screen reader.
const MANUAL_ACCESSIBILITY: &str = "AXManualAccessibility";

/// Chromium's list of an element's CSS classes.
const DOM_CLASS_LIST: &str = "AXDOMClassList";

/// Reads windows through Accessibility.
pub(crate) struct AxWindows {
    /// Read every attribute (the capture tool), not just the adapters'.
    full: bool,
}

impl AxWindows {
    /// The watcher's reader: the adapters' attributes only.
    pub(super) const fn live() -> Self {
        Self { full: false }
    }

    /// The capture tool's reader: every attribute a dump keeps.
    pub(crate) const fn capture() -> Self {
        Self { full: true }
    }
}

impl WindowReader for AxWindows {
    fn prepare(&self, app: CallApp, pid: Pid) {
        if app != CallApp::Meet {
            return;
        }
        let mut element = ax::UiElement::with_app_pid(pid.0);
        let name = cf::String::from_str(MANUAL_ACCESSIBILITY);
        let on = cf::Boolean::value_true();
        if let Err(error) = element.set_attr(ax::Attr::with_string(&name), on) {
            log::debug!(target: "deino_speaker_tags", "Chrome's web content stays hidden: {error}");
        }
    }

    fn windows(
        &self,
        pid: Pid,
        pick: &dyn Fn(&WindowSummary) -> bool,
    ) -> Result<Vec<AxNode>, AxError> {
        let app = ax::UiElement::with_app_pid(pid.0);
        app.set_messaging_timeout_secs(AX_TIMEOUT_SECS)
            .map_err(api_error)?;
        let children = app.children().map_err(api_error)?;
        let reader = Reader::new(self.full);
        let mut trees = Vec::new();
        for window in children
            .iter()
            .filter(|child| role(child) == WINDOW_ROLE)
            .take(MAX_WINDOWS)
        {
            let summary = WindowSummary {
                title: string(window, ax::attr::title()),
                document: string(window, ax::attr::document()),
            };
            if pick(&summary) {
                let mut budget = MAX_NODES;
                let mut tree = reader.tree(window, Walk::default(), &mut budget);
                tree.document = summary.document;
                trees.push(tree);
            }
        }
        Ok(trees)
    }
}

/// One read of a window, with its attribute names made once.
struct Reader {
    full: bool,
    dom_classes: cidre::arc::R<cf::String>,
}

impl Reader {
    fn new(full: bool) -> Self {
        Self {
            full,
            dom_classes: cf::String::from_str(DOM_CLASS_LIST),
        }
    }

    /// The element and what lies under it, within `budget` elements.
    fn tree(&self, element: &ax::UiElement, at: Walk, budget: &mut usize) -> AxNode {
        *budget = budget.saturating_sub(1);
        let mut node = self.node(element, at.web);
        let web = at.web || node.role == WEB_AREA_ROLE;
        if at.depth >= MAX_DEPTH || SKIPPED_ROLES.contains(&node.role.as_str()) {
            return node;
        }
        let Ok(children) = element.children() else {
            return node;
        };
        let below = Walk {
            depth: at.depth + 1,
            web,
        };
        for child in children.iter() {
            if *budget == 0 {
                break;
            }
            node.children.push(self.tree(child, below, budget));
        }
        node
    }

    /// One element's attributes.
    fn node(&self, element: &ax::UiElement, web: bool) -> AxNode {
        let classes = || {
            element
                .attr_value(ax::Attr::with_string(&self.dom_classes))
                .map(|value| dom::strings(&value))
                .unwrap_or_default()
        };
        AxNode {
            role: role(element),
            subrole: self
                .full
                .then(|| string(element, ax::attr::subrole()))
                .flatten(),
            title: string(element, ax::attr::title()),
            description: string(element, ax::attr::desc()),
            value: string(element, ax::attr::value()),
            identifier: self.full.then(|| string(element, ax::attr::id())).flatten(),
            dom_classes: if web { classes() } else { Vec::new() },
            document: None,
            children: Vec::new(),
        }
    }
}

/// Where an element sits in the walk.
#[derive(Clone, Copy, Debug, Default)]
struct Walk {
    /// Its depth under the window.
    depth: usize,
    /// It is web content (inside an `AXWebArea`).
    web: bool,
}

fn role(element: &ax::UiElement) -> String {
    element
        .role()
        .map(|role| role.to_string())
        .unwrap_or_default()
}

/// A text attribute, if the element has it as text.
fn string(element: &ax::UiElement, attr: &ax::Attr) -> Option<String> {
    let value = element.attr_value(attr).ok()?;
    value.try_as_string().map(ToString::to_string)
}

fn api_error(error: cidre::os::Error) -> AxError {
    AxError::Api(error.0.get())
}
