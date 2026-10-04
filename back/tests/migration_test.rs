mod common;

/// `set_updated_at()` (migration 001) must move `updated_at` on UPDATE —
/// every later table relies on it instead of setting the column by hand.
#[tokio::test]
async fn set_updated_at_moves_the_timestamp_on_update() {
    let pool = common::test_pool().await;
    // A temporary table lives in this one connection only, so the test
    // leaves nothing behind and cannot collide with a parallel run.
    let mut conn = pool.acquire().await.unwrap();

    sqlx::query(
        "CREATE TEMP TABLE probe (
             id INT PRIMARY KEY,
             note TEXT NOT NULL,
             updated_at TIMESTAMPTZ NOT NULL
         )",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER probe_set_updated_at
             BEFORE UPDATE ON probe
             FOR EACH ROW EXECUTE FUNCTION set_updated_at()",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    sqlx::query("INSERT INTO probe VALUES (1, 'before', '2000-01-01T00:00:00Z')")
        .execute(&mut *conn)
        .await
        .unwrap();

    sqlx::query("UPDATE probe SET note = 'after' WHERE id = 1")
        .execute(&mut *conn)
        .await
        .unwrap();

    let moved: bool =
        sqlx::query_scalar("SELECT updated_at > now() - interval '1 minute' FROM probe")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert!(moved, "updated_at should be bumped to now() by the trigger");
}
