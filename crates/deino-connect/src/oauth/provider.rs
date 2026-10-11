//! The OAuth providers Deino signs in to, and what each asks for.

/// An OAuth 2.0 authorization server for one kind of account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OAuthProvider {
    /// The provider's name in messages, for example "Google".
    pub name: &'static str,
    /// Where the user signs in and approves.
    pub authorize_url: &'static str,
    /// Where codes and refresh tokens become access tokens.
    pub token_url: &'static str,
    /// The permissions asked for: read-only wherever possible.
    pub scopes: &'static [&'static str],
    /// Extra query parameters of the sign-in page.
    pub authorize_params: &'static [(&'static str, &'static str)],
    /// Whether the token endpoint wants the client secret too (Google's
    /// desktop clients have one that is not really secret).
    pub sends_client_secret: bool,
    /// How the loopback redirect URI names this Mac, as the provider's app
    /// registration expects it.
    pub redirect_host: &'static str,
}

/// Google, for reading Google Calendar.
pub const GOOGLE: OAuthProvider = OAuthProvider {
    name: "Google",
    authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
    token_url: "https://oauth2.googleapis.com/token",
    scopes: &["https://www.googleapis.com/auth/calendar.readonly"],
    // `offline` returns a refresh token; `consent` returns one again on a
    // second connect, after a disconnect forgot the first.
    authorize_params: &[("access_type", "offline"), ("prompt", "consent")],
    sends_client_secret: true,
    redirect_host: "127.0.0.1",
};

/// Microsoft (work, school and personal accounts), for reading Outlook
/// calendars through Microsoft Graph.
pub const MICROSOFT: OAuthProvider = OAuthProvider {
    name: "Microsoft",
    authorize_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
    token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
    scopes: &["offline_access", "User.Read", "Calendars.Read"],
    authorize_params: &[("prompt", "select_account")],
    sends_client_secret: false,
    // Entra ID matches `http://localhost` registrations on any port.
    redirect_host: "localhost",
};
