//! session/stream-live-changes — the live socket, end to end: a real
//! server on a TCP port, real WebSocket clients, real `NOTIFY`s from
//! `campaigns::update_world`.

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use common::live::{FIXTURE, WAIT, next_of, open, open_player, ready, table};
use futures_util::SinkExt;
use promptus_back::campaigns;
use promptus_back::error::AppError;
use promptus_back::live::{LiveConfig, Signal};
use promptus_shared::story::FlagValue;
use serde_json::json;
use tokio_tungstenite::tungstenite::{self, Message};
use uuid::Uuid;

#[tokio::test]
async fn every_client_hears_each_committed_world_change_with_a_rising_version() {
    let t = table(LiveConfig::default()).await;
    let mut laptop = t.connect().await;
    let mut phone = t.connect().await;
    assert_eq!(ready(&mut laptop).await, json!({}));
    assert_eq!(ready(&mut phone).await, json!({}));

    // A write that fails rolls back, and its notification with it.
    let refused = campaigns::update_world(&t.pool, &t.gm, t.campaign, |_, w| {
        w.set_flag("count", FlagValue::Int(99));
        Err::<(), _>(AppError::BadRequest("NOPE"))
    })
    .await;
    assert!(refused.is_err());

    t.bump().await;
    t.bump().await;
    for ws in [&mut laptop, &mut phone] {
        for version in [1, 2] {
            let msg = next_of(ws, "changed").await;
            assert_eq!(
                msg,
                json!({ "type": "changed", "topic": "world", "version": version })
            );
        }
    }
}

#[tokio::test]
async fn a_client_cut_off_during_three_changes_catches_up_on_reconnect() {
    let t = table(LiveConfig::default()).await;
    t.bump().await;
    let mut phone = t.connect().await;
    let seen = ready(&mut phone).await;
    assert_eq!(seen, json!({ "world": 1 }));
    let held = t.refetch_world().await;

    // The phone loses its network; the table goes on.
    drop(phone);
    for _ in 0..3 {
        t.bump().await;
    }

    let mut phone = t.connect().await;
    let now = ready(&mut phone).await;
    assert_eq!(now, json!({ "world": 4 }), "the story never changed");
    // The client refetches every topic whose version moved, and lands on
    // exactly the table's state.
    assert_ne!(now["world"], seen["world"]);
    let world = t.refetch_world().await;
    assert_ne!(world, held);
    assert_eq!(world["flags"]["count"], 4, "{world}");
}

#[tokio::test]
async fn a_story_import_is_its_own_topic() {
    let t = table(LiveConfig::default()).await;
    let mut ws = t.connect().await;
    ready(&mut ws).await;
    let r = common::call(
        &t.router,
        Some(&t.token),
        "PUT",
        &format!("/api/campaigns/{}/import", t.campaign),
        Some(json!({ "yaml": FIXTURE })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let msg = next_of(&mut ws, "changed").await;
    assert_eq!(
        msg,
        json!({ "type": "changed", "topic": "story", "version": 1 })
    );
}

#[tokio::test]
async fn presence_follows_sockets_joining_and_leaving() {
    let t = table(LiveConfig::default()).await;
    let mut laptop = t.connect().await;
    ready(&mut laptop).await;
    let msg = next_of(&mut laptop, "presence").await;
    assert_eq!(
        msg,
        json!({ "type": "presence", "gmOnline": true, "players": [], "screens": [] })
    );
    assert!(t.hub.presence(t.campaign).gm_online);

    laptop.close(None).await.unwrap();
    let left = async {
        while t.hub.presence(t.campaign).gm_online {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    };
    tokio::time::timeout(WAIT, left)
        .await
        .expect("the GM leaves when the socket closes");
}

#[tokio::test]
async fn a_silent_client_times_out_and_leaves() {
    let t = table(LiveConfig {
        heartbeat: Duration::from_millis(50),
        timeout: Duration::from_millis(300),
        ..LiveConfig::default()
    })
    .await;
    let mut phone = t.connect().await;
    ready(&mut phone).await;
    assert!(t.hub.presence(t.campaign).gm_online);
    // The phone stops reading: it answers no ping, sends nothing, but
    // its TCP connection stays open — a phone whose network vanished.
    let gone = async {
        while t.hub.presence(t.campaign).gm_online {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    };
    tokio::time::timeout(WAIT, gone)
        .await
        .expect("the server drops a silent client");
    drop(phone);
}

#[tokio::test]
async fn a_client_ping_gets_a_pong() {
    let t = table(LiveConfig::default()).await;
    let mut ws = t.connect().await;
    ready(&mut ws).await;
    ws.send(Message::text(r#"{"type":"ping"}"#)).await.unwrap();
    next_of(&mut ws, "pong").await;
}

#[tokio::test]
async fn a_lagging_client_is_told_to_resync() {
    let t = table(LiveConfig {
        capacity: 4,
        ..LiveConfig::default()
    })
    .await;
    t.bump().await;
    let mut ws = t.connect().await;
    ready(&mut ws).await;
    // A burst the socket's task cannot keep up with: on this
    // single-threaded runtime it does not run until the loop is done.
    for version in 100..150 {
        t.hub.publish(
            t.campaign,
            Signal::Changed {
                topic: "world".into(),
                version,
            },
        );
    }
    let msg = next_of(&mut ws, "resync").await;
    // The versions come from the database, not from the lost signals.
    assert_eq!(msg["versions"], json!({ "world": 1 }));
}

/// session/invite-and-join wires the player socket: a seated player
/// hears what changed and shows up in the presence; a seat at another
/// table, or none, opens nothing.
#[tokio::test]
async fn a_seated_player_follows_their_campaign_and_only_it() {
    let t = table(LiveConfig::default()).await;
    let (marc, token) = t.seat("Marc").await;
    let mut phone = open_player(t.addr, Some(&token), t.campaign)
        .await
        .expect("a seated player opens the socket of their campaign");
    // Joining wrote his character and filled a seat: both topics
    // already have a version.
    let versions = ready(&mut phone).await;
    let mut topics: Vec<&str> = versions
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.split(':').next().unwrap())
        .collect();
    topics.sort_unstable();
    assert_eq!(topics, ["character", "table"], "{versions}");
    let msg = next_of(&mut phone, "presence").await;
    assert_eq!(msg["players"], json!([marc.to_string()]));

    t.bump().await;
    let msg = next_of(&mut phone, "changed").await;
    assert_eq!(
        msg,
        json!({ "type": "changed", "topic": "world", "version": 1 })
    );

    // A seat at another table of the same GM.
    let other = campaigns::create(
        &t.pool,
        &t.gm,
        &promptus_shared::story::from_yaml(FIXTURE).unwrap(),
    )
    .await
    .unwrap()
    .id;
    for (token, campaign) in [
        (Some(token.as_str()), other),
        (None, t.campaign),
        (Some("made-up"), t.campaign),
        // The GM's session is not a seat.
        (Some(t.token.as_str()), t.campaign),
    ] {
        match open_player(t.addr, token, campaign).await {
            Err(tungstenite::Error::Http(res)) => {
                assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
            }
            other => panic!("{token:?} on {campaign}: {:?}", other.map(|_| "open")),
        }
    }
    assert!(t.hub.presence(other).players.is_empty());
}

#[tokio::test]
async fn a_gm_cannot_follow_another_gms_campaign() {
    let t = table(LiveConfig::default()).await;
    let (_, marc) = common::signed_in_gm(&t.pool, "Marc").await;
    for (token, campaign, status) in [
        (Some(marc.as_str()), t.campaign, StatusCode::NOT_FOUND),
        (Some(marc.as_str()), Uuid::new_v4(), StatusCode::NOT_FOUND),
        (None, t.campaign, StatusCode::UNAUTHORIZED),
        (Some("made-up"), t.campaign, StatusCode::UNAUTHORIZED),
    ] {
        match open(t.addr, token, campaign).await {
            Err(tungstenite::Error::Http(res)) => assert_eq!(res.status(), status),
            other => panic!("{token:?} on {campaign}: {:?}", other.map(|_| "open")),
        }
    }
    assert!(!t.hub.presence(t.campaign).gm_online, "nobody joined");
}
