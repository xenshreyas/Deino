//! The `llm_calls` port and its SQLite implementation.

use async_trait::async_trait;
use deino_db::{Database, DbError};

use super::{CallOutcome, LlmCall};

/// Where usage records are kept.
#[async_trait]
pub trait LlmCallRepo: Send + Sync {
    /// Saves one record.
    async fn insert(&self, call: &LlmCall) -> Result<(), DbError>;
}

/// [`LlmCallRepo`] over Deino's SQLite database (`llm_calls`).
#[derive(Clone, Debug)]
pub struct SqliteLlmCallRepo {
    database: Database,
}

impl SqliteLlmCallRepo {
    /// A repository over an open, migrated database.
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

#[async_trait]
impl LlmCallRepo for SqliteLlmCallRepo {
    async fn insert(&self, call: &LlmCall) -> Result<(), DbError> {
        let (outcome, error_kind) = match call.outcome {
            CallOutcome::Ok => ("ok", None),
            CallOutcome::Error(kind) => ("error", Some(kind)),
            CallOutcome::Cancelled => ("cancelled", None),
        };
        let latency_ms = i64::try_from(call.latency_ms).unwrap_or(i64::MAX);
        sqlx::query!(
            "INSERT INTO llm_calls (id, started_at, route, provider, model, input_tokens,
                output_tokens, latency_ms, outcome, error_kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            call.id,
            call.started_at,
            call.route,
            call.provider,
            call.model,
            call.usage.input_tokens,
            call.usage.output_tokens,
            latency_ms,
            outcome,
            error_kind,
        )
        .execute(self.database.writer())
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use deino_domain::{FakeClock, LlmCallId};
    use deino_testkit::{at, test_db};

    use super::*;
    use crate::{LlmErrorKind, ProviderId, Route, Usage};

    fn call(outcome: CallOutcome) -> LlmCall {
        let clock = FakeClock::new(at("2026-10-09T09:00:00Z"));
        LlmCall {
            id: LlmCallId::generate(&clock),
            started_at: at("2026-10-09T09:00:00Z"),
            route: Route::ExtractActionItems,
            provider: ProviderId::OpenRouter,
            model: "anthropic/claude-haiku-5.5".into(),
            usage: Usage {
                input_tokens: 1200,
                output_tokens: 80,
            },
            latency_ms: 950,
            outcome,
        }
    }

    #[tokio::test]
    async fn insert_stores_metadata_with_the_route_and_provider_names() {
        let database = test_db().await;
        let repo = SqliteLlmCallRepo::new(database.clone());

        repo.insert(&call(CallOutcome::Error(LlmErrorKind::RateLimited)))
            .await
            .unwrap();

        let row: (String, String, i64, String, Option<String>) = sqlx::query_as(
            "SELECT route, provider, input_tokens, outcome, error_kind FROM llm_calls",
        )
        .fetch_one(database.reader())
        .await
        .unwrap();
        assert_eq!(
            row,
            (
                "extract.action_items".into(),
                "openrouter".into(),
                1200,
                "error".into(),
                Some("rate_limited".into())
            )
        );
    }

    #[tokio::test]
    async fn every_route_provider_and_error_kind_satisfies_the_check_constraints() {
        let database = test_db().await;
        let repo = SqliteLlmCallRepo::new(database);

        for route in Route::ALL {
            let mut record = call(CallOutcome::Ok);
            record.route = route;
            repo.insert(&record).await.unwrap();
        }
        for provider in ProviderId::ALL {
            let mut record = call(CallOutcome::Cancelled);
            record.provider = provider;
            repo.insert(&record).await.unwrap();
        }
        for kind in crate::error::LlmErrorKind::ALL {
            repo.insert(&call(CallOutcome::Error(*kind))).await.unwrap();
        }
    }

    #[tokio::test]
    async fn the_usage_list_order_is_an_index_scan() {
        let database = test_db().await;

        let steps: Vec<(i64, i64, i64, String)> = sqlx::query_as(
            "EXPLAIN QUERY PLAN SELECT * FROM llm_calls WHERE route = 'title' ORDER BY started_at DESC",
        )
        .fetch_all(database.reader())
        .await
        .unwrap();

        assert_eq!(
            steps[0].3,
            "SEARCH llm_calls USING INDEX idx_llm_calls_route_started_at (route=?)"
        );
    }
}
