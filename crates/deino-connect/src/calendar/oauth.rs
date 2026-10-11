//! The calendar providers' OAuth settings, in the integrations framework's
//! terms (`deino_integrations::OAuthSpec`, ADR 0029): one OAuth layer for
//! every connection Deino makes.

use deino_integrations::{ClientAuth, LoopbackHost, OAuthSpec, PortRule, RedirectSpec, Scope};

/// The path the sign-in page redirects to.
const CALLBACK_PATH: &str = "/callback";

/// Google, for reading Google Calendar: a "Desktop app" client, whose secret
/// Google treats as non-confidential and still wants with every token
/// request, redirecting to `127.0.0.1` on any port.
pub fn google() -> OAuthSpec {
    OAuthSpec {
        authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
        token_url: "https://oauth2.googleapis.com/token".into(),
        revoke_url: Some("https://oauth2.googleapis.com/revoke".into()),
        scopes: vec![Scope::new(
            "https://www.googleapis.com/auth/calendar.readonly",
            "Read your calendars and events",
        )],
        scope_param: "scope",
        scope_separator: ' ',
        // `offline` returns a refresh token; `consent` returns one again on a
        // second connect, after a disconnect forgot the first.
        extra_params: vec![("access_type", "offline"), ("prompt", "consent")],
        redirect: RedirectSpec {
            host: LoopbackHost::Ipv4,
            port: PortRule::Any,
            path: CALLBACK_PATH,
        },
        client_auth: ClientAuth::SecretInBody,
        app_setup_url: "https://console.cloud.google.com/apis/credentials".into(),
    }
}

/// Microsoft (work, school and personal accounts), for reading Outlook
/// calendars through Microsoft Graph: a public client (PKCE, no secret)
/// whose `http://localhost` registration matches any port.
pub fn microsoft() -> OAuthSpec {
    OAuthSpec {
        authorize_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".into(),
        token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".into(),
        // Entra ID has no RFC 7009 endpoint: access ends when the user
        // removes Deino from their account's apps.
        revoke_url: None,
        scopes: vec![
            Scope::new("offline_access", "Stay connected without signing in again"),
            Scope::new("User.Read", "Read your name and address"),
            Scope::new("Calendars.Read", "Read your calendars and events"),
        ],
        scope_param: "scope",
        scope_separator: ' ',
        extra_params: vec![("prompt", "select_account")],
        redirect: RedirectSpec {
            host: LoopbackHost::Localhost,
            port: PortRule::Any,
            path: CALLBACK_PATH,
        },
        client_auth: ClientAuth::Public,
        app_setup_url:
            "https://entra.microsoft.com/#view/Microsoft_AAD_RegisteredApps/ApplicationsListBlade"
                .into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use deino_integrations::oauth::{AuthorizationRequest, authorization_url};
    use deino_integrations::{OAuthClient, TokenEndpoint};
    use deino_net::{FakeNetClient, FakeReply, Request};
    use deino_secrets::SecretValue;
    use url::{Url, form_urlencoded};

    use super::*;

    fn form(request: &Request) -> BTreeMap<String, String> {
        let body = request.body.clone().unwrap_or_default();
        form_urlencoded::parse(&body).into_owned().collect()
    }

    #[test]
    fn the_sign_in_pages_ask_for_read_only_access_with_an_s256_challenge() {
        let redirect = Url::parse("http://127.0.0.1:50000/callback").unwrap();
        for (spec, scope) in [
            (
                google(),
                "https://www.googleapis.com/auth/calendar.readonly",
            ),
            (microsoft(), "offline_access User.Read Calendars.Read"),
        ] {
            let url = authorization_url(&AuthorizationRequest {
                spec: &spec,
                client_id: "id",
                redirect_uri: &redirect,
                state: "s",
                challenge: "c",
            })
            .unwrap();
            let query: BTreeMap<String, String> = url.query_pairs().into_owned().collect();
            assert_eq!(query["scope"], scope);
            assert_eq!(query["code_challenge_method"], "S256");
        }
    }

    #[tokio::test]
    async fn google_gets_its_client_secret_and_microsoft_refreshes_without_one() {
        let fake = FakeNetClient::default();
        fake.reply(FakeReply::ok(r#"{"access_token":"a","expires_in":3600}"#));
        fake.reply(FakeReply::ok(
            r#"{"access_token":"b","refresh_token":"r2"}"#,
        ));
        let app = |secret: Option<&str>| OAuthClient {
            client_id: "id".into(),
            client_secret: secret.map(|secret| SecretValue::parse(secret).unwrap()),
        };
        let (google_app, microsoft_app) = (app(Some("GOCSPX-1")), app(None));
        let refresh = SecretValue::parse("r1").unwrap();

        for (spec, app) in [(google(), &google_app), (microsoft(), &microsoft_app)] {
            let endpoint = TokenEndpoint {
                client: &fake,
                url: &spec.token_url,
                auth: spec.client_auth,
                app,
            };
            endpoint.refresh(&refresh).await.unwrap();
        }

        let sent = fake.requests();
        assert_eq!(form(&sent[0])["client_secret"], "GOCSPX-1");
        assert!(!form(&sent[1]).contains_key("client_secret"));
        assert_eq!(sent[1].url.host_str(), Some("login.microsoftonline.com"));
    }
}
