//! The list queries' access paths: each reads through its index, and the
//! filters' subqueries through theirs (PARITY_PLAN §3A "check with EXPLAIN
//! QUERY PLAN").

use deino_db::test_db;

use crate::testing::query_plan;

/// The plan of `sql` against a fresh, migrated database.
async fn plan(sql: &str) -> Vec<String> {
    query_plan(&test_db().await, sql).await
}

/// No step scans a whole table of user data.
fn assert_no_table_scan(plan: &[String]) {
    let scans: Vec<&String> = plan
        .iter()
        .filter(|step| {
            step.starts_with("SCAN ")
                && !matches!(step.as_str(), "SCAN subtree" | "SCAN CONSTANT ROW")
        })
        .collect();
    assert!(scans.is_empty(), "full scans in {plan:#?}");
}

#[tokio::test]
async fn the_status_path_searches_the_status_index() {
    let plan = plan(include_str!("queries/list_by_status.sql")).await;

    assert_eq!(
        plan.first().map(String::as_str),
        Some("SEARCH a USING INDEX idx_action_items_status_due_at (status=?)")
    );
    assert_no_table_scan(&plan);
}

#[tokio::test]
async fn the_person_path_searches_the_assignee_index() {
    let plan = plan(include_str!("queries/list_by_person.sql")).await;

    assert_eq!(
        plan.first().map(String::as_str),
        Some(
            "SEARCH a USING INDEX idx_action_items_assignee_status_due_at (assignee_person_id=? AND status=?)"
        )
    );
    assert_no_table_scan(&plan);
}

#[tokio::test]
async fn the_meeting_path_searches_the_meeting_index() {
    let plan = plan(include_str!("queries/list_by_meeting.sql")).await;

    assert_eq!(
        plan.first().map(String::as_str),
        Some("SEARCH a USING INDEX idx_action_items_meeting_id (meeting_id=?)")
    );
    assert_no_table_scan(&plan);
}

#[tokio::test]
async fn joins_and_filters_reach_rows_by_key() {
    let plan = plan(include_str!("queries/list_by_status.sql")).await;
    let expected = [
        "SEARCH m USING INDEX sqlite_autoindex_meetings_1 (id=?)",
        "SEARCH p USING INDEX sqlite_autoindex_people_1 (id=?)",
        "SEARCH s USING INDEX sqlite_autoindex_transcript_segments_1 (id=?)",
        "SEARCH mt USING COVERING INDEX sqlite_autoindex_meeting_tags_1 (meeting_id=? AND tag_id=?)",
        "SEARCH ma USING COVERING INDEX sqlite_autoindex_meeting_attendees_1 (meeting_id=?)",
        "SEARCH colleague USING INDEX sqlite_autoindex_people_1 (id=?)",
        "SEARCH mf USING COVERING INDEX idx_meeting_folders_folder_id (folder_id=?)",
        "SEARCH f USING INDEX idx_folders_parent_id (parent_id=?)",
    ];

    for step in expected {
        assert!(
            plan.iter().any(|line| line.starts_with(step)),
            "{step} missing from {plan:#?}"
        );
    }
}

#[tokio::test]
async fn one_item_is_read_by_its_primary_key() {
    let plan = plan(include_str!("queries/view.sql")).await;

    assert_eq!(
        plan.first().map(String::as_str),
        Some("SEARCH a USING INDEX sqlite_autoindex_action_items_1 (id=?)")
    );
}
