//! session/stream-live-changes — when the listener loses Postgres, every
//! socket is told to resync. Its own test binary: it cuts **every** live
//! listener of the test database, and a notification lost that way would
//! fail a test running alongside (test binaries run one after another).

mod common;

use common::live::{next_of, ready, table};
use promptus_back::live::LiveConfig;

#[tokio::test]
async fn sockets_resync_after_the_listener_loses_postgres() {
    let t = table(LiveConfig::default()).await;
    let mut ws = t.connect().await;
    ready(&mut ws).await;
    // Cut every live listener of the test database (a Postgres restart,
    // as far as they can tell). Other tests' listeners come back too.
    let cut: Vec<i32> = sqlx::query_scalar(
        "SELECT pid FROM pg_stat_activity
         WHERE datname = current_database() AND query ILIKE 'listen%'
           AND pid <> pg_backend_pid()",
    )
    .fetch_all(&t.pool)
    .await
    .unwrap();
    assert!(!cut.is_empty(), "no listener found");
    for pid in cut {
        sqlx::query("SELECT pg_terminate_backend($1)")
            .bind(pid)
            .execute(&t.pool)
            .await
            .unwrap();
    }
    next_of(&mut ws, "resync").await;
    // And it listens again: the next change gets through.
    t.bump().await;
    let msg = next_of(&mut ws, "changed").await;
    assert_eq!(msg["version"], 1);
}
