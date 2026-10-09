//! The vocabulary of detection: processes, apps, meeting platforms and the
//! kind of meeting a prompt announces.

use deino_domain::BundleId;
use serde::Serialize;

/// A Unix process id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pid(pub i32);

/// One Core Audio client process, as the audio system reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioProcess {
    /// The process.
    pub pid: Pid,
    /// `kAudioProcessPropertyBundleID`; helpers report their own
    /// (`com.google.Chrome.helper`), daemons often none.
    pub bundle_id: Option<BundleId>,
    /// The process is capturing audio (`kAudioProcessPropertyIsRunningInput`).
    pub running_input: bool,
    /// The process is playing audio (`kAudioProcessPropertyIsRunningOutput`).
    pub running_output: bool,
}

/// An app as the user knows it: the outermost `.app` bundle a process
/// belongs to, so a browser or Electron helper maps to its parent app.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppIdentity {
    /// The bundle identifier, which the ignore list and cooldowns key on.
    pub bundle_id: BundleId,
    /// The display name, for example "Google Chrome".
    pub name: String,
}

/// An app capturing from the microphone, after attribution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MicClient {
    /// The app the capturing process belongs to.
    pub app: AppIdentity,
    /// The capturing process (a helper's own pid, not the parent's).
    pub pid: Pid,
    /// The app also plays audio: both sides of a conversation are live.
    pub full_duplex: bool,
}

/// The meeting service a detection belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    /// Zoom (app or `zoom.us` in a browser).
    Zoom,
    /// Microsoft Teams.
    Teams,
    /// Google Meet in a browser.
    GoogleMeet,
    /// Cisco Webex.
    Webex,
    /// A Slack huddle.
    Slack,
    /// FaceTime.
    FaceTime,
    /// An iPhone call relayed to the Mac.
    Phone,
    /// A WhatsApp call.
    WhatsApp,
    /// Discord.
    Discord,
    /// Any other app holding the microphone.
    Other,
}

impl Platform {
    /// What the prompt calls a meeting on this platform (Granola §2.3).
    pub const fn kind(self) -> MeetingKind {
        match self {
            Self::Slack => MeetingKind::Huddle,
            Self::FaceTime | Self::Phone | Self::WhatsApp => MeetingKind::Call,
            Self::Zoom
            | Self::Teams
            | Self::GoogleMeet
            | Self::Webex
            | Self::Discord
            | Self::Other => MeetingKind::Meeting,
        }
    }
}

/// The prompt's title: "Meeting detected", "Huddle detected" or "Call detected".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MeetingKind {
    /// A meeting (Zoom, Teams, Meet, Webex, Discord, anything else).
    Meeting,
    /// A Slack huddle.
    Huddle,
    /// A FaceTime, phone or WhatsApp call.
    Call,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_is_a_huddle_and_personal_calls_are_calls() {
        assert_eq!(Platform::Slack.kind(), MeetingKind::Huddle);
        for call in [Platform::FaceTime, Platform::Phone, Platform::WhatsApp] {
            assert_eq!(call.kind(), MeetingKind::Call);
        }
        for meeting in [
            Platform::Zoom,
            Platform::GoogleMeet,
            Platform::Discord,
            Platform::Other,
        ] {
            assert_eq!(meeting.kind(), MeetingKind::Meeting);
        }
    }
}
