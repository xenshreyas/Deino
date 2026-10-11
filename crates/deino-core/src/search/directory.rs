//! [`FilterDirectory`] over SQLite: the names `@` filters can refer to.

use async_trait::async_trait;

use super::index::FilterDirectory;
use super::{FilterChip, FilterKind, SearchError, SqliteSearchIndex};

/// The most candidates [`FilterDirectory::resolve`] looks at for one name.
const RESOLVE_CANDIDATES: u32 = 1;

#[async_trait]
impl FilterDirectory for SqliteSearchIndex {
    async fn suggest(
        &self,
        prefix: &str,
        kind: Option<FilterKind>,
        limit: u32,
    ) -> Result<Vec<FilterChip>, SearchError> {
        let escaped = escape_like(prefix.trim());
        let starts = format!("{escaped}%");
        let later_word = format!("% {escaped}%");
        let chips = sqlx::query_file_as!(
            FilterChip,
            "src/search/queries/filters.sql",
            kind,
            starts,
            later_word,
            limit
        )
        .fetch_all(self.database.reader())
        .await?;
        Ok(chips)
    }

    async fn resolve(
        &self,
        kind: Option<FilterKind>,
        name: &str,
    ) -> Result<Option<FilterChip>, SearchError> {
        let exact = escape_like(name.trim());
        let chip = sqlx::query_file_as!(
            FilterChip,
            "src/search/queries/filters.sql",
            kind,
            exact,
            None::<String>,
            RESOLVE_CANDIDATES
        )
        .fetch_optional(self.database.reader())
        .await?;
        Ok(chip)
    }

    async fn knows_domain(&self, domain: &str) -> Result<bool, SearchError> {
        let known = sqlx::query_file_scalar!("src/search/queries/known_domain.sql", domain)
            .fetch_one(self.database.reader())
            .await?;
        Ok(known)
    }
}

/// `text` with LIKE's wildcards (and the escape character) escaped by `\`.
fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_like;

    #[test]
    fn like_wildcards_typed_by_the_user_match_literally() {
        assert_eq!(escape_like("100%_off\\"), "100\\%\\_off\\\\");
    }
}
