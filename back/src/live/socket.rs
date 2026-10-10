//! One live socket: what it sends, and how long it waits.
//!
//! Server → client, JSON text frames:
//! - `{"type":"ready","versions":{"world":7,"story":2}}` — first, on
//!   every connection: the current version of each topic;
//! - `{"type":"changed","topic":"world","version":8}`;
//! - `{"type":"presence","gmOnline":true,"players":["<id>",…]}`;
//! - `{"type":"activity","author":"token","revision":12}` — the GM's
//!   sockets only: a program wrote with the GM's token;
//! - `{"type":"resync","versions":{…}}` — signals were lost (this socket
//!   lagged, or the listener lost Postgres): compare again, as on `ready`;
//! - `{"type":"pong"}` — the answer to a client `{"type":"ping"}`.
//!
//! A GM-only topic (`super::visible_to`) and the activity never reach a
//! player's or a screen's socket, whether as a change or in the versions.
//!
//! The client pings because a browser cannot see WebSocket ping frames,
//! and a phone that lost its network keeps a socket that looks open: a
//! missing pong is how it notices. The server pings too and drops a
//! client it has not heard for `LiveConfig::timeout` — its presence ends.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;
use uuid::Uuid;

use super::{Activity, LiveHub, Presence, Signal, Viewer, versions, visible_to};

/// Clients only send pings: anything bigger is not ours.
const MAX_CLIENT_MESSAGE: usize = 1024;

#[derive(Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum Out {
    Ready {
        versions: BTreeMap<String, i64>,
    },
    Changed {
        topic: String,
        version: i64,
    },
    Presence {
        #[serde(flatten)]
        presence: Arc<Presence>,
    },
    Activity {
        #[serde(flatten)]
        activity: Activity,
    },
    Resync {
        versions: BTreeMap<String, i64>,
    },
    Pong,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum In {
    Ping,
}

/// Accept the socket of `viewer` on `campaign`. The caller has already
/// checked that `viewer` may follow `campaign`.
pub fn upgrade(
    ws: WebSocketUpgrade,
    pool: PgPool,
    hub: LiveHub,
    campaign: Uuid,
    viewer: Viewer,
) -> Response {
    ws.max_message_size(MAX_CLIENT_MESSAGE)
        .on_upgrade(move |socket| run(socket, pool, hub, campaign, viewer))
}

async fn send(socket: &mut WebSocket, out: &Out, within: Duration) -> bool {
    let Ok(text) = serde_json::to_string(out) else {
        return false;
    };
    matches!(
        tokio::time::timeout(within, socket.send(Message::text(text))).await,
        Ok(Ok(()))
    )
}

async fn close(mut socket: WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: reason.into(),
        })))
        .await;
}

/// The versions `viewer` may see.
fn visible(viewer: &Viewer, mut versions: BTreeMap<String, i64>) -> BTreeMap<String, i64> {
    versions.retain(|topic, _| visible_to(viewer, topic));
    versions
}

/// What `viewer` is sent for `signal`, if anything; `None` from the outer
/// option ends the socket.
async fn out_for(
    viewer: &Viewer,
    signal: Result<Signal, RecvError>,
    pool: &PgPool,
    campaign: Uuid,
) -> Option<Option<Out>> {
    let gm = matches!(viewer, Viewer::Gm(_));
    Some(match signal {
        Ok(Signal::Changed { topic, version }) => {
            visible_to(viewer, &topic).then_some(Out::Changed { topic, version })
        }
        Ok(Signal::Presence(presence)) => Some(Out::Presence { presence }),
        Ok(Signal::Activity(activity)) => gm.then_some(Out::Activity { activity }),
        Ok(Signal::Resync) | Err(RecvError::Lagged(_)) => match versions(pool, campaign).await {
            Ok(versions) => Some(Out::Resync {
                versions: visible(viewer, versions),
            }),
            Err(_) => return None,
        },
        Err(RecvError::Closed) => return None,
    })
}

async fn run(mut socket: WebSocket, pool: PgPool, hub: LiveHub, campaign: Uuid, viewer: Viewer) {
    let config = hub.config();
    // Join first: from here every signal is queued for this socket, so
    // nothing can slip between the versions read below and the stream.
    let mut member = hub.join(campaign, viewer.clone());
    if !hub.wait_listening(config.listener_wait).await {
        // 1013 "try again later": the client reconnects with its backoff.
        close(socket, close_code::AGAIN, "listener unavailable").await;
        return;
    }
    let Ok(snapshot) = versions(&pool, campaign).await else {
        close(socket, close_code::ERROR, "database unavailable").await;
        return;
    };
    if !send(
        &mut socket,
        &Out::Ready {
            versions: visible(&viewer, snapshot),
        },
        config.timeout,
    )
    .await
    {
        return;
    }

    let mut last_heard = Instant::now();
    let mut beat = tokio::time::interval_at(Instant::now() + config.heartbeat, config.heartbeat);
    loop {
        tokio::select! {
            signal = member.recv() => {
                let Some(out) = out_for(&viewer, signal, &pool, campaign).await else {
                    break;
                };
                if let Some(out) = out
                    && !send(&mut socket, &out, config.timeout).await
                {
                    break;
                }
            }
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                Some(Ok(Message::Text(text))) => {
                    last_heard = Instant::now();
                    if matches!(serde_json::from_str::<In>(&text), Ok(In::Ping))
                        && !send(&mut socket, &Out::Pong, config.timeout).await
                    {
                        break;
                    }
                }
                Some(Ok(_)) => last_heard = Instant::now(),
            },
            _ = beat.tick() => {
                if last_heard.elapsed() > config.timeout {
                    break;
                }
                let ping = socket.send(Message::Ping(Default::default()));
                if !matches!(tokio::time::timeout(config.timeout, ping).await, Ok(Ok(()))) {
                    break;
                }
            }
        }
    }
    // `member` drops here: the room hears that this viewer left.
}
