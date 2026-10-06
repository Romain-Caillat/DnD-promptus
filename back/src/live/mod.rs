//! session/stream-live-changes — the live channel of a campaign.
//!
//! The channel carries **invalidations, never data** (`MEMORY.md` §3):
//! "the world of this campaign changed, it is now at version 7", and who
//! is connected. A client that hears a topic changed refetches it through
//! the HTTP API, so the player projection stays the only path from the
//! database to a player.
//!
//! How a change travels:
//! 1. a write bumps its topic's counter and calls `pg_notify` **in its
//!    own transaction** ([`touch`]), so the signal leaves only if the
//!    write commits;
//! 2. one listener task per server `LISTEN`s on [`CHANNEL`]
//!    ([`listener`]) and hands each notification to the [`LiveHub`];
//! 3. the hub fans it out to the campaign's sockets through a tokio
//!    broadcast channel per campaign ([`socket`]).
//!
//! Catching up: on every (re)connection the socket first sends the
//! current version of every topic of the campaign; the client refetches
//! each topic whose version differs from what it last saw. A socket that
//! falls behind its broadcast channel, or every socket after the
//! listener lost Postgres (notifications may have been dropped), gets a
//! `resync` with the versions again — the same catch-up.

pub mod listener;
pub mod socket;

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, Postgres, Transaction};
use tokio::sync::{broadcast, watch};
use uuid::Uuid;

use crate::error::AppError;

/// The Postgres channel every change is notified on.
pub const CHANNEL: &str = "promptus_live";

/// Something a client may hold and must refetch when it changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Topic {
    /// The living world of the campaign (`campaigns.world`).
    World,
    /// The prepared story (`campaigns.story`).
    Story,
    /// One character sheet.
    Character(Uuid),
    /// Who sits at the table and where each character stands (the GM's
    /// seat list): touched on join, removal and every character write
    /// (`players::touch_character`).
    Table,
}

impl Topic {
    /// The topic as stored and sent: `world`, `story`, `character:<id>`,
    /// `table`.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::World => "world".to_string(),
            Self::Story => "story".to_string(),
            Self::Character(id) => format!("character:{id}"),
            Self::Table => "table".to_string(),
        }
    }
}

/// The payload of one notification on [`CHANNEL`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Notice {
    campaign: Uuid,
    topic: String,
    version: i64,
}

/// Bump `topic` of `campaign` and notify the listener, inside `tx`: the
/// notification is delivered when `tx` commits, and never if it rolls
/// back. Returns the new version.
///
/// Call it from every write a client may be displaying — today
/// `campaigns::save_world` and `campaigns::save_story`.
///
/// # Errors
///
/// Fails on a database error.
pub async fn touch(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    topic: &Topic,
) -> Result<i64, AppError> {
    let topic = topic.key();
    let version: i64 = sqlx::query_scalar(
        "INSERT INTO live_versions (campaign_id, topic, version) VALUES ($1, $2, 1)
         ON CONFLICT (campaign_id, topic)
         DO UPDATE SET version = live_versions.version + 1
         RETURNING version",
    )
    .bind(campaign)
    .bind(&topic)
    .fetch_one(&mut **tx)
    .await?;
    let notice = Notice {
        campaign,
        topic,
        version,
    };
    let payload =
        serde_json::to_string(&notice).map_err(|e| AppError::internal("live notice", e))?;
    sqlx::query("SELECT pg_notify($1, $2)")
        .bind(CHANNEL)
        .bind(payload)
        .execute(&mut **tx)
        .await?;
    Ok(version)
}

/// The current version of every topic of `campaign` that ever changed.
///
/// # Errors
///
/// Fails on a database error.
pub async fn versions(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
) -> Result<BTreeMap<String, i64>, AppError> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT topic, version FROM live_versions WHERE campaign_id = $1")
            .bind(campaign)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().collect())
}

/// Who opens a live socket. Each kind of caller has its own route and
/// extractor; they all end in [`socket::upgrade`] with one of these.
///
/// The GM's is `GET /api/campaigns/{id}/live`; a player's is
/// `GET /api/play/{campaign}/live`, behind `require_player`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Viewer {
    /// The GM who owns the campaign (checked by the route).
    Gm(Uuid),
    /// A player of `campaign` (checked by the player extractor).
    Player { campaign: Uuid, player: Uuid },
}

/// Who is connected to a campaign right now. Ids only: a client maps
/// player ids to nicknames with what it already fetched.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    pub gm_online: bool,
    /// Each connected player once, however many devices they use.
    pub players: Vec<Uuid>,
}

/// What the hub tells a campaign's sockets.
#[derive(Debug, Clone)]
pub enum Signal {
    Changed {
        topic: String,
        version: i64,
    },
    Presence(Arc<Presence>),
    /// Notifications may have been lost: send the versions again.
    Resync,
}

/// Timings and sizes of the live channel.
#[derive(Debug, Clone, Copy)]
pub struct LiveConfig {
    /// How often a socket pings its client.
    pub heartbeat: Duration,
    /// A client silent for this long is gone: its presence ends.
    pub timeout: Duration,
    /// Signals buffered per campaign before a slow socket must resync.
    pub capacity: usize,
    /// How long a new socket waits for the listener to be up before
    /// giving up (the listener is down only while Postgres is).
    pub listener_wait: Duration,
}

impl Default for LiveConfig {
    fn default() -> Self {
        Self {
            heartbeat: Duration::from_secs(15),
            timeout: Duration::from_secs(45),
            capacity: 64,
            listener_wait: Duration::from_secs(5),
        }
    }
}

struct Room {
    signals: broadcast::Sender<Signal>,
    members: HashMap<u64, Viewer>,
}

impl Room {
    fn presence(&self) -> Presence {
        let mut players: Vec<Uuid> = self
            .members
            .values()
            .filter_map(|v| match v {
                Viewer::Player { player, .. } => Some(*player),
                Viewer::Gm(_) => None,
            })
            .collect();
        players.sort_unstable();
        players.dedup();
        Presence {
            gm_online: self.members.values().any(|v| matches!(v, Viewer::Gm(_))),
            players,
        }
    }
}

struct Inner {
    config: LiveConfig,
    rooms: Mutex<HashMap<Uuid, Room>>,
    next_member: AtomicU64,
    listening: watch::Sender<bool>,
}

/// The in-memory side of the live channel: one room per campaign with a
/// socket connected, holding its broadcast channel and its presence.
#[derive(Clone)]
pub struct LiveHub {
    inner: Arc<Inner>,
}

impl LiveHub {
    #[must_use]
    pub fn new(config: LiveConfig) -> Self {
        Self {
            inner: Arc::new(Inner {
                config,
                rooms: Mutex::new(HashMap::new()),
                next_member: AtomicU64::new(0),
                listening: watch::Sender::new(false),
            }),
        }
    }

    #[must_use]
    pub fn config(&self) -> LiveConfig {
        self.inner.config
    }

    fn rooms(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, Room>> {
        // A panic while holding the lock leaves the map consistent (every
        // change is a single insert or remove), so keep serving.
        self.inner
            .rooms
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Enter the room of `campaign`: the returned membership receives
    /// every signal sent from now on, everyone (this member included) is
    /// told the new presence, and dropping it leaves the room.
    #[must_use]
    pub fn join(&self, campaign: Uuid, viewer: Viewer) -> Membership {
        let id = self.inner.next_member.fetch_add(1, Ordering::Relaxed);
        let mut rooms = self.rooms();
        let room = rooms.entry(campaign).or_insert_with(|| Room {
            signals: broadcast::Sender::new(self.inner.config.capacity),
            members: HashMap::new(),
        });
        let signals = room.signals.subscribe();
        room.members.insert(id, viewer);
        let _ = room
            .signals
            .send(Signal::Presence(Arc::new(room.presence())));
        Membership {
            hub: self.clone(),
            campaign,
            id,
            signals,
        }
    }

    fn leave(&self, campaign: Uuid, id: u64) {
        let mut rooms = self.rooms();
        let Some(room) = rooms.get_mut(&campaign) else {
            return;
        };
        room.members.remove(&id);
        if room.members.is_empty() {
            rooms.remove(&campaign);
        } else {
            let _ = room
                .signals
                .send(Signal::Presence(Arc::new(room.presence())));
        }
    }

    /// Who is connected to `campaign` now.
    #[must_use]
    pub fn presence(&self, campaign: Uuid) -> Presence {
        self.rooms()
            .get(&campaign)
            .map(Room::presence)
            .unwrap_or_default()
    }

    /// Send `signal` to every socket of `campaign`, if any is connected.
    pub fn publish(&self, campaign: Uuid, signal: Signal) {
        if let Some(room) = self.rooms().get(&campaign) {
            let _ = room.signals.send(signal);
        }
    }

    /// Ask every connected socket to send its versions again.
    pub fn resync_all(&self) {
        for room in self.rooms().values() {
            let _ = room.signals.send(Signal::Resync);
        }
    }

    /// Relay one notification payload from [`CHANNEL`].
    fn dispatch(&self, payload: &str) {
        match serde_json::from_str::<Notice>(payload) {
            Ok(n) => self.publish(
                n.campaign,
                Signal::Changed {
                    topic: n.topic,
                    version: n.version,
                },
            ),
            Err(e) => tracing::warn!(%e, payload, "ignoring a malformed live notice"),
        }
    }

    fn set_listening(&self, up: bool) {
        self.inner.listening.send_replace(up);
    }

    /// Wait until the listener `LISTEN`s, at most `within`. True when it
    /// does. A socket waits for this before reading versions, so no
    /// change can fall between its snapshot and the notifications.
    pub async fn wait_listening(&self, within: Duration) -> bool {
        let mut rx = self.inner.listening.subscribe();
        matches!(
            tokio::time::timeout(within, rx.wait_for(|up| *up)).await,
            Ok(Ok(_))
        )
    }
}

/// One socket's place in a campaign room. Leaves on drop, whatever ended
/// the socket (close, timeout, error or a server shutting the task down).
pub struct Membership {
    hub: LiveHub,
    campaign: Uuid,
    id: u64,
    signals: broadcast::Receiver<Signal>,
}

impl Membership {
    /// The next signal for this member, or `Lagged` when it fell more
    /// than `capacity` signals behind.
    ///
    /// # Errors
    ///
    /// `RecvError::Lagged` after missed signals.
    pub async fn recv(&mut self) -> Result<Signal, broadcast::error::RecvError> {
        self.signals.recv().await
    }
}

impl Drop for Membership {
    fn drop(&mut self) {
        self.hub.leave(self.campaign, self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_counts_each_player_once_and_ends_with_the_last_socket() {
        let hub = LiveHub::new(LiveConfig::default());
        let campaign = Uuid::new_v4();
        let marc = Uuid::new_v4();
        let phone = hub.join(
            campaign,
            Viewer::Player {
                campaign,
                player: marc,
            },
        );
        let laptop = hub.join(
            campaign,
            Viewer::Player {
                campaign,
                player: marc,
            },
        );
        assert_eq!(
            hub.presence(campaign),
            Presence {
                gm_online: false,
                players: vec![marc]
            }
        );
        let gm = hub.join(campaign, Viewer::Gm(Uuid::new_v4()));
        assert!(hub.presence(campaign).gm_online);
        drop(phone);
        assert_eq!(hub.presence(campaign).players, vec![marc]);
        drop(laptop);
        assert!(hub.presence(campaign).players.is_empty());
        drop(gm);
        assert!(hub.rooms().is_empty(), "an empty room is removed");
    }

    #[test]
    fn a_notice_reaches_only_its_campaign() {
        let hub = LiveHub::new(LiveConfig::default());
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let mut on_a = hub.join(a, Viewer::Gm(Uuid::new_v4()));
        let mut on_b = hub.join(b, Viewer::Gm(Uuid::new_v4()));
        // Drain the join presences.
        while on_a.signals.try_recv().is_ok() {}
        while on_b.signals.try_recv().is_ok() {}
        hub.dispatch(&format!(
            r#"{{"campaign":"{a}","topic":"world","version":3}}"#
        ));
        hub.dispatch("not json");
        match on_a.signals.try_recv() {
            Ok(Signal::Changed { topic, version }) => {
                assert_eq!((topic.as_str(), version), ("world", 3))
            }
            other => panic!("{other:?}"),
        }
        assert!(on_a.signals.try_recv().is_err());
        assert!(on_b.signals.try_recv().is_err());
    }
}
