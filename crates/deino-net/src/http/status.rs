//! [`Status`]: an HTTP status code.

use std::fmt;

/// An HTTP status code.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Status(u16);

impl Status {
    /// `200 OK`.
    pub const OK: Self = Self(200);
    /// `206 Partial Content`: the answer to a `Range` request.
    pub const PARTIAL_CONTENT: Self = Self(206);
    /// `301 Moved Permanently`.
    pub const MOVED_PERMANENTLY: Self = Self(301);
    /// `302 Found`.
    pub const FOUND: Self = Self(302);
    /// `303 See Other`.
    pub const SEE_OTHER: Self = Self(303);
    /// `307 Temporary Redirect`.
    pub const TEMPORARY_REDIRECT: Self = Self(307);
    /// `308 Permanent Redirect`.
    pub const PERMANENT_REDIRECT: Self = Self(308);
    /// `404 Not Found`.
    pub const NOT_FOUND: Self = Self(404);
    /// `416 Range Not Satisfiable`: the requested range starts past the end.
    pub const RANGE_NOT_SATISFIABLE: Self = Self(416);

    /// A status from its code.
    pub const fn new(code: u16) -> Self {
        Self(code)
    }

    /// The numeric code.
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Whether the code is 2xx.
    pub const fn is_success(self) -> bool {
        self.0 >= 200 && self.0 < 300
    }

    /// Whether the code is a redirect that carries a `Location`.
    pub const fn is_redirect(self) -> bool {
        matches!(self.0, 301 | 302 | 303 | 307 | 308)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "HTTP {}", self.0)
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_location_redirects_are_followed() {
        assert!(Status::FOUND.is_redirect());
        assert!(!Status::new(304).is_redirect());
        assert!(Status::PARTIAL_CONTENT.is_success());
        assert!(!Status::RANGE_NOT_SATISFIABLE.is_success());
    }
}
