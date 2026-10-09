//! The bundle-identifier tables behind [`super::role_of`].
//!
//! Meeting apps and the weak pair follow Muesli's `MeetingCandidateResolver`
//! (MIT); the deny categories follow anarlog's `plugins/detect/src/policy.rs`
//! (MIT); browsers follow anarlog's `meeting-apps.ts`. See
//! `THIRD_PARTY_NOTICES.md`.

use crate::model::Platform;

/// How much a meeting app's microphone use says on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Strength {
    /// Holding the microphone means a call.
    Strong,
    /// Runs in the background with audio (Slack, WhatsApp): a call only in
    /// full duplex, or with other evidence.
    Weak,
}

/// Dedicated meeting and calling apps.
pub(super) const MEETING_APPS: &[(&str, Platform, Strength)] = &[
    ("us.zoom.xos", Platform::Zoom, Strength::Strong),
    ("us.zoom.ZoomPhone", Platform::Zoom, Strength::Strong),
    ("com.microsoft.teams2", Platform::Teams, Strength::Strong),
    ("com.microsoft.teams", Platform::Teams, Strength::Strong),
    (
        "com.webex.meetingmanager",
        Platform::Webex,
        Strength::Strong,
    ),
    (
        "com.cisco.webexmeetingsapp",
        Platform::Webex,
        Strength::Strong,
    ),
    ("Cisco-Systems.Spark", Platform::Webex, Strength::Strong),
    ("com.apple.FaceTime", Platform::FaceTime, Strength::Strong),
    ("com.apple.mobilephone", Platform::Phone, Strength::Strong),
    ("com.hnc.Discord", Platform::Discord, Strength::Strong),
    ("com.tinyspeck.slackmacgap", Platform::Slack, Strength::Weak),
    ("net.whatsapp.WhatsApp", Platform::WhatsApp, Strength::Weak),
    ("desktop.WhatsApp", Platform::WhatsApp, Strength::Weak),
];

/// Web browsers. Chromium browsers and Safari expose the page URL through
/// Accessibility; the others are still attributed, without a URL.
pub(super) const BROWSERS: &[&str] = &[
    "com.google.Chrome",
    "com.google.Chrome.beta",
    "com.google.Chrome.canary",
    "com.brave.Browser",
    "company.thebrowser.Browser",
    "company.thebrowser.dia",
    "com.microsoft.edgemac",
    "com.apple.Safari",
    "com.apple.SafariTechnologyPreview",
    "org.chromium.Chromium",
    "com.vivaldi.Vivaldi",
    "com.operasoftware.Opera",
    "org.mozilla.firefox",
    "app.zen-browser.zen",
];

/// Apps whose microphone use is never a meeting: dictation, IDEs and
/// terminals, screen recorders, AI assistants, audio tools, and Apple's
/// speech service, which "also processes ordinary playback" (OpenWhispr).
pub(super) const DENIED: &[&str] = &[
    // Deino itself, every build (the self check also matches its own pid).
    "com.xenshreyas.deino",
    // Dictation.
    "com.pais.handy",
    "com.electron.wispr-flow",
    "com.seewillow.WillowMac",
    "com.superduper.superwhisper",
    "com.prakashjoshipax.VoiceInk",
    "com.goodsnooze.macwhisper",
    "com.descript.beachcube",
    "com.apple.VoiceMemos",
    "com.electron.aqua-voice",
    "com.gizmolabs.openwhispr",
    "com.muesli.app",
    // IDEs and terminals.
    "dev.warp.Warp-Stable",
    "com.exafunction.windsurf",
    "com.microsoft.VSCode",
    "com.todesktop.230313mzl4w4u92",
    // Screen recorders.
    "so.cap.desktop",
    "com.timpler.screenstudio",
    "com.loom.desktop",
    "com.obsproject.obs-studio",
    "pl.maketheweb.cleanshotx",
    "com.getcleanshot.app-setapp",
    "com.wulkano.kap",
    "net.telestream.screenflow10",
    "com.techsmith.camtasia2024",
    "com.TechSmith.Snagit2024",
    "com.apple.QuickTimePlayerX",
    "com.apple.screenshot.launcher",
    // AI assistants.
    "com.openai.chat",
    "com.openai.codex",
    "com.anthropic.claudefordesktop",
    // Other audio users.
    "com.raycast.macos",
    "com.apple.garageband10",
    "com.apple.Sound-Settings.extension",
    "com.apple.CoreSpeech",
    "com.apple.siri",
];
