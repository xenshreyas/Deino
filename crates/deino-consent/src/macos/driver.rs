//! [`AxDriver`]: acts on a meeting app's chat through Accessibility.
//!
//! Each call finds the element again: the app's window with the same title
//! and address, then the path's children one by one, and checks the role
//! the snapshot had. Every request has a short messaging timeout, so a hung
//! app cannot stall the monitor. Text goes in through `AXValue` and buttons
//! are pressed with `AXPress`; no key events are posted anywhere. Nothing
//! read or written here is logged.

use cidre::arc::Retain;
use cidre::{arc, ax, cf};
use deino_speaker_tags::{AxError, WindowSummary, tree::WINDOW_ROLE};

use crate::error::ChatError;
use crate::ports::{ChatDriver, Target};

/// How long one Accessibility request may take, in seconds.
const AX_TIMEOUT_SECS: f32 = 0.25;

/// Acts through `AXUIElement`.
pub(super) struct AxDriver;

impl ChatDriver for AxDriver {
    fn focus(&self, target: &Target) -> Result<(), ChatError> {
        let mut element = element(target)?;
        element
            .set_attr(ax::attr::focused(), cf::Boolean::value_true())
            .map_err(|error| api(target, error))
    }

    fn value(&self, target: &Target) -> Result<String, ChatError> {
        let element = element(target)?;
        Ok(string(&element, ax::attr::value()).unwrap_or_default())
    }

    fn set_value(&self, target: &Target, text: &str) -> Result<(), ChatError> {
        let mut element = element(target)?;
        let text = cf::String::from_str(text);
        element
            .set_attr(ax::attr::value(), &text)
            .map_err(|error| api(target, error))
    }

    fn press(&self, target: &Target) -> Result<(), ChatError> {
        element(target)?
            .perform_action(ax::action::press())
            .map_err(|error| api(target, error))
    }
}

/// The live element `target` names, if it is still there with its role.
fn element(target: &Target) -> Result<arc::R<ax::UiElement>, ChatError> {
    let app = ax::UiElement::with_app_pid(target.pid.0);
    app.set_messaging_timeout_secs(AX_TIMEOUT_SECS)
        .map_err(|error| api(target, error))?;
    let mut element = window(&app, &target.window).ok_or(ChatError::Moved(target.app))?;
    for &index in target.path.indices() {
        let children = element.children().map_err(|error| api(target, error))?;
        let child = children
            .iter()
            .nth(index)
            .ok_or(ChatError::Moved(target.app))?
            .retained();
        element = child;
    }
    if role(&element) == target.role {
        Ok(element)
    } else {
        Err(ChatError::Moved(target.app))
    }
}

/// The app's window with `summary`'s title and address.
fn window(app: &ax::UiElement, summary: &WindowSummary) -> Option<arc::R<ax::UiElement>> {
    let children = app.children().ok()?;
    children
        .iter()
        .filter(|child| role(child) == WINDOW_ROLE)
        .find(|window| {
            string(window, ax::attr::title()) == summary.title
                && string(window, ax::attr::document()) == summary.document
        })
        .map(Retain::retained)
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

fn api(target: &Target, error: cidre::os::Error) -> ChatError {
    ChatError::Ax {
        app: target.app,
        source: AxError::Api(error.0.get()),
    }
}
