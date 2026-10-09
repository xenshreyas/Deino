//! Helpers shared by this crate's fakes and repository tests.
//!
//! Compiled for tests and behind `test-support` only; nothing here ships.

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locks a fake's state. A panic in another test thread cannot leave a
/// `Vec` half-updated, so a poisoned lock is still safe to use.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The access path SQLite chooses for `sql`: one `detail` line per step of
/// `EXPLAIN QUERY PLAN`, for example `SEARCH people USING INDEX ...`.
///
/// Numbered parameters (`?1`, `?2`, ...) are bound to NULL; the plan does not
/// depend on their values.
#[cfg(test)]
pub(crate) async fn query_plan(database: &deino_db::Database, sql: &str) -> Vec<String> {
    let explain = format!("EXPLAIN QUERY PLAN {sql}");
    let mut query = sqlx::query_as::<_, (i64, i64, i64, String)>(&explain);
    for _ in 0..parameter_count(sql) {
        query = query.bind(None::<String>);
    }
    let steps = query.fetch_all(database.reader()).await.unwrap();
    steps.into_iter().map(|(.., detail)| detail).collect()
}

/// The highest `?N` in `sql`.
#[cfg(test)]
fn parameter_count(sql: &str) -> usize {
    sql.split('?')
        .skip(1)
        .filter_map(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<usize>().ok()
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_count_is_the_highest_numbered_parameter() {
        assert_eq!(parameter_count("SELECT 1"), 0);
        assert_eq!(parameter_count("WHERE a = ?2 AND b = ?1 AND c = ?2"), 2);
    }
}
