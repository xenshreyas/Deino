//! [`Method`]: the HTTP methods Deino sends, as stored in `egress_log.method`.

use serde::{Deserialize, Serialize};

/// An HTTP method. Stored in `egress_log.method`, whose `CHECK` list matches
/// [`Method::ALL`].
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type, sqlx::Type,
)]
#[serde(rename_all = "UPPERCASE")]
#[sqlx(rename_all = "UPPERCASE")]
pub enum Method {
    /// `GET`.
    Get,
    /// `HEAD`.
    Head,
    /// `POST`.
    Post,
    /// `PUT`.
    Put,
    /// `PATCH`.
    Patch,
    /// `DELETE`.
    Delete,
}

impl Method {
    /// Every method, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Get,
        Self::Head,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
    ];

    /// The method as sent on the wire and stored.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_and_wire_names_agree() {
        for method in Method::ALL {
            let json = serde_json::to_value(method).unwrap();
            assert_eq!(json.as_str(), Some(method.as_str()));
        }
    }
}
