//! The one task that `LISTEN`s on Postgres for the whole server.
//!
//! It holds one connection of the pool for as long as it runs. When that
//! connection drops (Postgres restarted, network cut), notifications sent
//! meanwhile are lost for good: the task reconnects with a backoff and,
//! once it listens again, tells every socket to resync.

use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgListener;
use tokio::task::JoinHandle;

use super::{CHANNEL, LiveHub};

const RETRY_MIN: Duration = Duration::from_millis(500);
const RETRY_MAX: Duration = Duration::from_secs(10);

/// Start listening for `hub`. The task runs until the runtime stops.
pub fn spawn(pool: PgPool, hub: LiveHub) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut retry = RETRY_MIN;
        let mut ever_listened = false;
        loop {
            match listen(&pool).await {
                Ok(mut listener) => {
                    hub.set_listening(true);
                    if ever_listened {
                        // Whatever was notified while we were away is gone.
                        hub.resync_all();
                    }
                    ever_listened = true;
                    retry = RETRY_MIN;
                    loop {
                        match listener.try_recv().await {
                            Ok(Some(n)) => hub.dispatch(n.payload()),
                            Ok(None) => {
                                tracing::warn!("live listener lost its Postgres connection");
                                break;
                            }
                            Err(e) => {
                                tracing::warn!(%e, "live listener failed");
                                break;
                            }
                        }
                    }
                    hub.set_listening(false);
                }
                Err(e) => tracing::warn!(%e, "live listener cannot listen, retrying"),
            }
            tokio::time::sleep(retry).await;
            retry = (retry * 2).min(RETRY_MAX);
        }
    })
}

async fn listen(pool: &PgPool) -> Result<PgListener, sqlx::Error> {
    let mut listener = PgListener::connect_with(pool).await?;
    listener.listen(CHANNEL).await?;
    Ok(listener)
}
