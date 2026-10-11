//! Which video-call service a join link belongs to: a table of host and path
//! rules, one row per link shape seen in real invitations.

use HostMatch::{Domain, Exact};
use MeetingService as S;
use PathMatch::{Contains, NotRoot, Prefix};
use deino_domain::ConferencePlatform;
use url::Url;

/// A video-call service recognised from its join link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetingService {
    /// Zoom, including Zoom for Government.
    Zoom,
    /// Google Meet.
    GoogleMeet,
    /// Microsoft Teams (work, personal and government clouds).
    Teams,
    /// Cisco Webex.
    Webex,
    /// A Slack huddle.
    SlackHuddle,
    /// `GoTo` Meeting.
    GoTo,
    /// Jitsi Meet and 8x8.
    Jitsi,
    /// Whereby.
    Whereby,
    /// Amazon Chime.
    Chime,
    /// `BlueJeans`.
    BlueJeans,
    /// Around.
    Around,
    /// Skype.
    Skype,
    /// `RingCentral` Video.
    RingCentral,
    /// `FaceTime` links.
    FaceTime,
    /// Discord.
    Discord,
    /// Dialpad Meetings.
    Dialpad,
    /// Lark (Feishu) video meetings.
    Lark,
    /// Zoho Meeting.
    ZohoMeeting,
    /// Pop.
    Pop,
}

impl MeetingService {
    /// The service as `calendar_events.platform` stores it: the services
    /// Deino treats specially have their own value, the rest are `other`.
    pub const fn platform(self) -> ConferencePlatform {
        match self {
            Self::Zoom => ConferencePlatform::Zoom,
            Self::GoogleMeet => ConferencePlatform::GoogleMeet,
            Self::Teams => ConferencePlatform::Teams,
            Self::Webex => ConferencePlatform::Webex,
            Self::SlackHuddle => ConferencePlatform::Slack,
            _ => ConferencePlatform::Other,
        }
    }

    /// The service whose join link `url` is, if any.
    pub fn of(url: &Url) -> Option<Self> {
        match url.scheme() {
            "zoommtg" | "zoomus" => return Some(Self::Zoom),
            "msteams" => return Some(Self::Teams),
            "http" | "https" => {}
            _ => return None,
        }
        let host = url.host_str()?.to_ascii_lowercase();
        let path = url.path().to_ascii_lowercase();
        RULES
            .iter()
            .find(|rule| rule.host.matches(&host) && rule.path.matches(&path))
            .map(|rule| rule.service)
    }
}

/// Which hosts a rule covers.
enum HostMatch {
    /// Exactly this host.
    Exact(&'static str),
    /// This domain and every subdomain of it (`us02web.zoom.us`).
    Domain(&'static str),
}

impl HostMatch {
    fn matches(&self, host: &str) -> bool {
        match self {
            Self::Exact(exact) => host == *exact,
            Self::Domain(domain) => {
                host == *domain
                    || host
                        .strip_suffix(domain)
                        .is_some_and(|prefix| prefix.ends_with('.'))
            }
        }
    }
}

/// Which paths of a host are join links (not a help page or a home page).
enum PathMatch {
    /// The path starts with one of these.
    Prefix(&'static [&'static str]),
    /// The path contains one of these.
    Contains(&'static [&'static str]),
    /// Any path but the root.
    NotRoot,
}

impl PathMatch {
    fn matches(&self, path: &str) -> bool {
        match self {
            Self::Prefix(prefixes) => prefixes.iter().any(|prefix| path.starts_with(prefix)),
            Self::Contains(parts) => parts.iter().any(|part| path.contains(part)),
            Self::NotRoot => path.trim_matches('/').len() > 1,
        }
    }
}

/// One link shape.
struct Rule {
    service: MeetingService,
    host: HostMatch,
    path: PathMatch,
}

/// A table row.
const fn rule(service: MeetingService, host: HostMatch, path: PathMatch) -> Rule {
    Rule {
        service,
        host,
        path,
    }
}

/// Zoom's join paths: meeting, personal room, start, webinar, web client.
const ZOOM_PATHS: PathMatch = Prefix(&["/j/", "/my/", "/s/", "/w/", "/wc/"]);
/// Teams' join paths: classic `meetup-join`, the short `meet/<id>` links.
const TEAMS_PATHS: PathMatch = Prefix(&["/l/meetup-join/", "/meet/", "/l/meet/", "/v2/"]);

/// Every recognised link shape, checked in order.
const RULES: &[Rule] = &[
    rule(S::Zoom, Domain("zoom.us"), ZOOM_PATHS),
    rule(S::Zoom, Domain("zoomgov.com"), ZOOM_PATHS),
    rule(S::Zoom, Domain("zoom.com"), ZOOM_PATHS),
    rule(S::GoogleMeet, Exact("meet.google.com"), NotRoot),
    rule(S::GoogleMeet, Exact("g.co"), Prefix(&["/meet/"])),
    rule(S::Teams, Exact("teams.microsoft.com"), TEAMS_PATHS),
    rule(S::Teams, Exact("teams.live.com"), TEAMS_PATHS),
    rule(S::Teams, Exact("teams.cloud.microsoft"), TEAMS_PATHS),
    rule(S::Teams, Domain("teams.microsoft.us"), TEAMS_PATHS),
    rule(
        S::Webex,
        Domain("webex.com"),
        Contains(&[
            "/meet/",
            "j.php",
            "m.php",
            "/join",
            "joinservice",
            "/meeting/",
        ]),
    ),
    rule(S::SlackHuddle, Domain("slack.com"), Contains(&["/huddle/"])),
    rule(S::GoTo, Exact("meet.goto.com"), NotRoot),
    rule(
        S::GoTo,
        Domain("gotomeeting.com"),
        Contains(&["/join", "meetingid"]),
    ),
    rule(S::GoTo, Domain("gotomeet.me"), NotRoot),
    rule(S::Jitsi, Exact("meet.jit.si"), NotRoot),
    rule(S::Jitsi, Exact("8x8.vc"), NotRoot),
    rule(S::Whereby, Domain("whereby.com"), NotRoot),
    rule(S::Chime, Exact("chime.aws"), NotRoot),
    rule(S::Chime, Exact("app.chime.aws"), Prefix(&["/meetings/"])),
    rule(S::BlueJeans, Domain("bluejeans.com"), NotRoot),
    rule(S::Around, Domain("around.co"), Prefix(&["/r/"])),
    rule(S::Skype, Exact("join.skype.com"), NotRoot),
    rule(
        S::RingCentral,
        Exact("meetings.ringcentral.com"),
        Prefix(&["/j/"]),
    ),
    rule(
        S::RingCentral,
        Exact("v.ringcentral.com"),
        Prefix(&["/join/"]),
    ),
    rule(S::RingCentral, Exact("video.ringcentral.com"), NotRoot),
    rule(S::FaceTime, Exact("facetime.apple.com"), Prefix(&["/join"])),
    rule(S::Discord, Exact("discord.gg"), NotRoot),
    rule(
        S::Discord,
        Exact("discord.com"),
        Prefix(&["/invite/", "/channels/"]),
    ),
    rule(S::Dialpad, Exact("meetings.dialpad.com"), NotRoot),
    rule(S::Lark, Domain("larksuite.com"), Prefix(&["/j/"])),
    rule(S::Lark, Domain("feishu.cn"), Prefix(&["/j/"])),
    rule(S::ZohoMeeting, Domain("meeting.zoho.com"), NotRoot),
    rule(S::ZohoMeeting, Domain("meeting.zoho.eu"), NotRoot),
    rule(S::Pop, Exact("pop.com"), Prefix(&["/j/"])),
];
