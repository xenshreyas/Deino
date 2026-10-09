//! [`HostPattern`]: which hosts a service may reach.

use std::borrow::Cow;

use url::Url;

/// A host, or a family of hosts, that a service may reach. Ports are not
/// part of a pattern: a service may use any port on its hosts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostPattern {
    /// Exactly this host, for example `api.anthropic.com` or `[::1]`.
    Exact(Cow<'static, str>),
    /// Any subdomain of this domain (not the domain itself), for example
    /// `hf.co` for `cas-bridge.xethub.hf.co`.
    SubdomainsOf(&'static str),
}

impl HostPattern {
    /// A pattern matching exactly `host` (stored lower case).
    pub fn exact(host: &str) -> Self {
        Self::Exact(Cow::Owned(host.to_ascii_lowercase()))
    }

    /// Whether `host` (as `Url::host_str` gives it, lower case) matches.
    pub fn matches(&self, host: &str) -> bool {
        match self {
            Self::Exact(exact) => host.eq_ignore_ascii_case(exact),
            Self::SubdomainsOf(domain) => host
                .len()
                .checked_sub(domain.len() + 1)
                .and_then(|dot| host.get(dot..))
                .and_then(|tail| tail.strip_prefix('.'))
                .is_some_and(|tail| tail.eq_ignore_ascii_case(domain)),
        }
    }

    /// The pattern as the UI shows it: `api.openai.com` or `*.hf.co`.
    pub fn display(&self) -> String {
        match self {
            Self::Exact(exact) => exact.to_string(),
            Self::SubdomainsOf(domain) => format!("*.{domain}"),
        }
    }
}

/// The host of `url`, lower case, or `None` unless it is an absolute
/// `http`/`https` URL with a host. Custom endpoints in the settings are
/// turned into host patterns with it.
pub fn host_of(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.host_str().map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_patterns_match_one_host_in_any_case() {
        let pattern = HostPattern::exact("api.openai.com");
        assert!(pattern.matches("API.openai.com"));
        assert!(!pattern.matches("evil-api.openai.com"));
        assert!(!pattern.matches("api.openai.com.evil.test"));
    }

    #[test]
    fn subdomain_patterns_need_a_dot_boundary_and_exclude_the_domain() {
        let pattern = HostPattern::SubdomainsOf("hf.co");
        assert!(pattern.matches("cas-bridge.xethub.hf.co"));
        assert!(!pattern.matches("hf.co"));
        assert!(!pattern.matches("evilhf.co"));
        assert!(!pattern.matches("co"));
    }

    #[test]
    fn hosts_come_only_from_absolute_web_urls() {
        assert_eq!(
            host_of("http://LocalHost:11434/v1").as_deref(),
            Some("localhost")
        );
        assert_eq!(host_of("http://[::1]:8000").as_deref(), Some("[::1]"));
        assert_eq!(host_of("ftp://example.com"), None);
        assert_eq!(host_of("localhost:11434"), None);
    }
}
