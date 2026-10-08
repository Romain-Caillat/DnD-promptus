//! Live sockets for tests: a real server on a TCP port and real
//! WebSocket clients (`session/stream-live-changes`).

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::{StatusCode, header};
use futures_util::StreamExt;
use promptus_back::auth::guard::CurrentGm;
use promptus_back::campaigns;
use promptus_back::live::{LiveConfig, LiveHub};
use promptus_shared::story::{FlagValue, from_yaml};
use serde_json::Value;
use sqlx::PgPool;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

pub const FIXTURE: &str = include_str!("../../../content/fixtures/phare-de-kerbrume.yaml");
pub const WAIT: Duration = Duration::from_secs(10);

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Table {
    pub pool: PgPool,
    pub hub: LiveHub,
    pub addr: SocketAddr,
    pub router: axum::Router,
    pub gm: CurrentGm,
    pub token: String,
    pub campaign: Uuid,
}

pub async fn table(config: LiveConfig) -> Table {
    let pool = super::test_pool_sized(6).await;
    let (router, hub) = super::app_live(pool.clone(), config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let served = router.clone();
    tokio::spawn(async move { axum::serve(listener, served).await.unwrap() });
    assert!(
        hub.wait_listening(WAIT).await,
        "the listener never listened"
    );
    let (id, token) = super::signed_in_gm(&pool, "Romain").await;
    let gm = CurrentGm {
        id,
        display_name: "Romain".into(),
    };
    let campaign = campaigns::create(&pool, &gm, &from_yaml(FIXTURE).unwrap())
        .await
        .unwrap()
        .id;
    Table {
        pool,
        hub,
        addr,
        router,
        gm,
        token,
        campaign,
    }
}

/// Open the live socket of `campaign` with the GM cookie `token`.
pub async fn open(
    addr: SocketAddr,
    token: Option<&str>,
    campaign: Uuid,
) -> Result<Socket, tungstenite::Error> {
    let mut req = format!("ws://{addr}/api/campaigns/{campaign}/live")
        .into_client_request()
        .unwrap();
    if let Some(token) = token {
        req.headers_mut().insert(
            header::COOKIE,
            format!("promptus_gm={token}").parse().unwrap(),
        );
    }
    tokio_tungstenite::connect_async(req)
        .await
        .map(|(ws, _)| ws)
}

/// Open a player's live socket of `campaign` with the device `token`.
pub async fn open_player(
    addr: SocketAddr,
    token: Option<&str>,
    campaign: Uuid,
) -> Result<Socket, tungstenite::Error> {
    let mut req = format!("ws://{addr}/api/play/{campaign}/live")
        .into_client_request()
        .unwrap();
    if let Some(token) = token {
        req.headers_mut().insert(
            header::COOKIE,
            format!("promptus_player={token}").parse().unwrap(),
        );
    }
    tokio_tungstenite::connect_async(req)
        .await
        .map(|(ws, _)| ws)
}

/// Open a paired shared screen's live socket with its device `token`.
pub async fn open_screen(addr: SocketAddr, token: &str) -> Result<Socket, tungstenite::Error> {
    let mut req = format!("ws://{addr}/api/tv/live")
        .into_client_request()
        .unwrap();
    req.headers_mut().insert(
        header::COOKIE,
        format!("promptus_screen={token}").parse().unwrap(),
    );
    tokio_tungstenite::connect_async(req)
        .await
        .map(|(ws, _)| ws)
}

impl Table {
    /// Seat `nickname` at the table; returns the player id and token.
    pub async fn seat(&self, nickname: &str) -> (Uuid, String) {
        let campaign = self.campaign.to_string();
        let code = super::invite_code(&self.router, &self.token, &campaign).await;
        let r = super::join(&self.router, &code, nickname, "player").await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        let id = Uuid::parse_str(r.body["data"]["me"]["id"].as_str().unwrap()).unwrap();
        (id, r.player_token().unwrap())
    }

    pub async fn connect(&self) -> Socket {
        open(self.addr, Some(&self.token), self.campaign)
            .await
            .expect("the GM opens the socket of their campaign")
    }

    /// One world write through the real write path: a counter flag.
    pub async fn bump(&self) {
        campaigns::update_world(&self.pool, &self.gm, self.campaign, |_, w| {
            let n = match w.flags.get("count") {
                Some(FlagValue::Int(n)) => *n,
                _ => 0,
            };
            w.set_flag("count", FlagValue::Int(n + 1));
            Ok(())
        })
        .await
        .unwrap();
    }

    /// What a client refetches: the campaign through the HTTP API.
    pub async fn refetch_world(&self) -> Value {
        let r = super::call(
            &self.router,
            Some(&self.token),
            "GET",
            &format!("/api/campaigns/{}", self.campaign),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK);
        r.body["data"]["world"].clone()
    }
}

/// The next JSON message, skipping control frames (tungstenite answers
/// the server's pings while it reads).
pub async fn next(ws: &mut Socket) -> Value {
    loop {
        let msg = tokio::time::timeout(WAIT, ws.next())
            .await
            .expect("a message in time")
            .expect("the socket is open")
            .expect("a frame");
        if let Message::Text(text) = msg {
            return serde_json::from_str(&text).unwrap();
        }
    }
}

/// The next message of `kind`, skipping the others (presence, or a
/// resync caused by another test restarting the listeners).
pub async fn next_of(ws: &mut Socket, kind: &str) -> Value {
    loop {
        let msg = next(ws).await;
        if msg["type"] == kind {
            return msg;
        }
    }
}

pub async fn ready(ws: &mut Socket) -> Value {
    let msg = next(ws).await;
    assert_eq!(msg["type"], "ready", "{msg}");
    msg["versions"].clone()
}
