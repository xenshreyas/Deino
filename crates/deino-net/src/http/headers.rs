//! [`Headers`]: header names and values, looked up without regard to case.

/// Header names (stored lower case) and values, in insertion order.
///
/// Values may hold API keys, so `Headers` is not `Debug` and nothing logs it.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Headers(Vec<(String, String)>);

impl Headers {
    /// Sets `name` to `value`, replacing any value it had.
    pub fn insert(&mut self, name: &str, value: impl Into<String>) {
        let name = name.to_ascii_lowercase();
        self.0.retain(|(existing, _)| *existing != name);
        self.0.push((name, value.into()));
    }

    /// The value of `name`, if set.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Keeps only the headers whose (lower-case) name `keep` accepts.
    pub fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.0.retain(|(name, _)| keep(name));
    }

    /// Every header as `(lower-case name, value)`.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }
}

impl<N: AsRef<str>, V: Into<String>> FromIterator<(N, V)> for Headers {
    fn from_iter<I: IntoIterator<Item = (N, V)>>(pairs: I) -> Self {
        let mut headers = Self::default();
        for (name, value) in pairs {
            headers.insert(name.as_ref(), value);
        }
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_case_insensitive_and_set_once() {
        let mut headers = Headers::default();
        headers.insert("Content-Type", "text/plain");
        headers.insert("content-type", "application/json");

        assert_eq!(headers.get("CONTENT-TYPE"), Some("application/json"));
        assert_eq!(headers.iter().count(), 1);
    }
}
