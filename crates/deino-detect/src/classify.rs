//! What an app's microphone use means: a meeting app, a browser, an app that
//! is never a meeting, or something unknown.
//!
//! Matching is by bundle identifier, and a helper (`com.google.Chrome.helper`)
//! takes its parent's role, so attribution misses never turn Chrome's audio
//! helper into an "unknown app".

mod tables;

use deino_domain::BundleId;

use crate::model::Platform;
use tables::{BROWSERS, DENIED, MEETING_APPS, Strength};

/// What detection makes of an app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppRole {
    /// A dedicated meeting app. A `weak` one (Slack, WhatsApp) keeps audio
    /// open in the background, so its microphone use counts only in full
    /// duplex or with other evidence.
    Meeting {
        /// The service.
        platform: Platform,
        /// Needs full duplex or corroboration.
        weak: bool,
    },
    /// A web browser: its meetings are told apart by URL.
    Browser,
    /// Never a meeting: dictation, IDEs, screen recorders, assistants, Deino.
    Denied,
    /// Anything else: a meeting only after a sustained hold of the microphone.
    Unknown,
}

/// The role of the app with `bundle_id`.
pub fn role_of(bundle_id: &BundleId) -> AppRole {
    if let Some((platform, strength)) = meeting_app(bundle_id) {
        return AppRole::Meeting {
            platform,
            weak: strength == Strength::Weak,
        };
    }
    if matches_any(bundle_id, BROWSERS) {
        return AppRole::Browser;
    }
    if matches_any(bundle_id, DENIED) {
        return AppRole::Denied;
    }
    AppRole::Unknown
}

/// The browser `bundle_id` is, or is a helper of.
pub fn browser_of(bundle_id: &BundleId) -> Option<BundleId> {
    BROWSERS
        .iter()
        .filter_map(|browser| BundleId::try_from(*browser).ok())
        .find(|browser| bundle_id == browser || bundle_id.is_helper_of(browser))
}

fn meeting_app(bundle_id: &BundleId) -> Option<(Platform, Strength)> {
    MEETING_APPS
        .iter()
        .find(|(id, _, _)| matches(bundle_id, id))
        .map(|(_, platform, strength)| (*platform, *strength))
}

fn matches_any(bundle_id: &BundleId, table: &[&str]) -> bool {
    table.iter().any(|id| matches(bundle_id, id))
}

/// `bundle_id` is `id` or one of its helpers.
fn matches(bundle_id: &BundleId, id: &str) -> bool {
    BundleId::try_from(id).is_ok_and(|id| *bundle_id == id || bundle_id.is_helper_of(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(id: &str) -> AppRole {
        role_of(&BundleId::try_from(id).unwrap())
    }

    fn meeting(platform: Platform, weak: bool) -> AppRole {
        AppRole::Meeting { platform, weak }
    }

    #[test]
    fn real_meeting_app_bundle_ids_map_to_their_platform() {
        let table = [
            ("us.zoom.xos", meeting(Platform::Zoom, false)),
            ("com.microsoft.teams2", meeting(Platform::Teams, false)),
            ("com.microsoft.teams", meeting(Platform::Teams, false)),
            (
                "com.cisco.webexmeetingsapp",
                meeting(Platform::Webex, false),
            ),
            ("Cisco-Systems.Spark", meeting(Platform::Webex, false)),
            ("com.apple.FaceTime", meeting(Platform::FaceTime, false)),
            ("com.hnc.Discord", meeting(Platform::Discord, false)),
            ("com.tinyspeck.slackmacgap", meeting(Platform::Slack, true)),
            ("net.whatsapp.WhatsApp", meeting(Platform::WhatsApp, true)),
        ];
        for (id, expected) in table {
            assert_eq!(role(id), expected, "{id}");
        }
    }

    #[test]
    fn helpers_take_their_parents_role() {
        assert_eq!(role("com.google.Chrome.helper"), AppRole::Browser);
        assert_eq!(
            role("com.tinyspeck.slackmacgap.helper"),
            meeting(Platform::Slack, true)
        );
        assert_eq!(role("com.xenshreyas.deino.smoke-cap06"), AppRole::Denied);
    }

    #[test]
    fn browsers_are_browsers_and_their_helpers_resolve_to_them() {
        for id in [
            "com.google.Chrome",
            "com.brave.Browser",
            "company.thebrowser.Browser",
            "com.microsoft.edgemac",
            "com.apple.Safari",
        ] {
            assert_eq!(role(id), AppRole::Browser, "{id}");
        }
        let helper = BundleId::try_from("com.brave.Browser.helper").unwrap();
        assert_eq!(browser_of(&helper).unwrap().as_str(), "com.brave.Browser");
        assert_eq!(
            browser_of(&BundleId::try_from("us.zoom.xos").unwrap()),
            None
        );
    }

    #[test]
    fn dictation_ides_recorders_assistants_and_core_speech_are_denied() {
        for id in [
            "com.pais.handy",
            "com.superduper.superwhisper",
            "com.microsoft.VSCode",
            "com.apple.QuickTimePlayerX",
            "com.anthropic.claudefordesktop",
            "com.apple.CoreSpeech",
            "com.xenshreyas.deino",
        ] {
            assert_eq!(role(id), AppRole::Denied, "{id}");
        }
    }

    #[test]
    fn anything_else_is_unknown() {
        assert_eq!(role("com.apple.PhotoBooth"), AppRole::Unknown);
        assert_eq!(role("process.ffmpeg"), AppRole::Unknown);
        // A prefix that is not followed by a dot is a different app.
        assert_eq!(role("us.zoom.xosx"), AppRole::Unknown);
    }
}
