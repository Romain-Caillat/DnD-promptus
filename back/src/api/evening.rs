//! The GM's evening (all behind `require_gm`; another GM's campaign
//! answers 404 like a missing one):
//! - `GET  /api/campaigns/{id}/session` → the whole live screen: the
//!   session and its lobby, the current scene in full, the fronts, the
//!   requests (with past rulings for the same situation), the journal
//!   (hidden lines included), what the next scenes need, the spotlight;
//! - `POST /api/campaigns/{id}/session` → open the lobby (201);
//! - `POST /api/campaigns/{id}/session/start` → the session goes live;
//! - `POST /api/campaigns/{id}/session/end` → `{ recap, previously }`;
//! - `POST /api/campaigns/{id}/session/reveal` → a scene, a clue, an
//!   NPC, a front's clock, a resolved scene (`evening::scenes::Reveal`);
//! - `PUT /api/campaigns/{id}/session/music` → `{ track }` (or null);
//! - `POST /api/campaigns/{id}/session/journal` → a promise, a debt, a
//!   key item, a note;
//! - `POST /api/campaigns/{id}/session/requests/{request}` → the GM's
//!   answer (`evening::requests::Decision`);
//! - `POST /api/campaigns/{id}/session/spotlight/{player}` → a moment
//!   given;
//! - `PUT  /api/campaigns/{id}/hooks/{hook}/played` → `{ played }`;
//! - `GET  /api/campaigns/{id}/knowledge?ref=…&situation=…` → what the
//!   table knows about a story id, and past rulings like a situation;
//! - `GET  /api/campaigns/{id}/sessions` → the chronicle;
//! - `PUT  /api/campaigns/{id}/sessions/{session}/recap` → edit the recap;
//! - `POST /api/campaigns/{id}/sessions/{session}/recap-draft` → the
//!   co-GM's draft of both recaps (counted AI call; nothing saved);
//! - `GET  /api/campaigns/{id}/sessions/{session}/feedback` → answers and
//!   measures on one screen;
//! - `PUT  /api/campaigns/{id}/sessions/{session}/changes` → `{ text }`;
//! - `GET  /api/campaigns/{id}/ai` → the AI budget, spending and calls;
//! - `POST /api/campaigns/{id}/session/copilot` → a co-GM draft (counted
//!   AI call), then `…/copilot/{draft}/show` (the GM's edited text to the
//!   table) or `…/copilot/{draft}/dismiss`.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use promptus_shared::story::recap;
use promptus_shared::story::{Campaign, Node, WorldState};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use super::body::Body;
use crate::ai::{self, LlmRequest, ledger, templates};
use crate::auth::guard::{CurrentGm, owned_by};
use crate::campaigns::{self, CampaignRow};
use crate::copilot::{self, drafts};
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::evening::requests::{self, Decision, Request};
use crate::evening::scenes::{self, MusicChoice, Note, Reveal};
use crate::evening::session::{self, Ending, Session};
use crate::evening::{feedback, spotlight};
use crate::players;
use crate::state::AppState;

fn parse_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::NotFound("NOT_FOUND"))
}

async fn owned_row(state: &AppState, gm: &CurrentGm, raw: &str) -> Result<CampaignRow, AppError> {
    owned_by(campaigns::find(&state.pool, parse_id(raw)?).await?, gm)
}

/// A clue of the current scene, found or not.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SceneClue {
    id: String,
    text: String,
    discovery: String,
    found: bool,
    revelation: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SceneNpc {
    id: String,
    name: String,
    title: String,
    role: String,
    roleplay: String,
    wants: String,
    hides: String,
    met: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitView {
    to: String,
    label: String,
    title: String,
    visited: bool,
}

/// The current scene as the GM runs it: the node whole, and what it
/// links to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmScene {
    node: Node,
    clues: Vec<SceneClue>,
    npcs: Vec<SceneNpc>,
    exits: Vec<ExitView>,
}

fn gm_scene(story: &Campaign, world: &WorldState) -> Option<GmScene> {
    let node = story.node(world.current_node.as_deref()?)?;
    Some(GmScene {
        clues: story
            .clues
            .iter()
            .filter(|c| c.node == node.id)
            .map(|c| SceneClue {
                id: c.id.clone(),
                text: c.text.clone(),
                discovery: c.discovery.clone(),
                found: world.found_clues.contains(&c.id),
                revelation: c.revelation.clone(),
            })
            .collect(),
        npcs: node
            .npcs
            .iter()
            .filter_map(|p| {
                let n = story.npc(&p.npc)?;
                Some(SceneNpc {
                    id: n.id.clone(),
                    name: n.name.clone(),
                    title: n.title.clone(),
                    role: p.role.clone(),
                    roleplay: n.roleplay.clone(),
                    wants: n.wants.clone(),
                    hides: n.hides.clone(),
                    met: world.revealed.contains(&n.id),
                })
            })
            .collect(),
        exits: node
            .exits
            .iter()
            .map(|e| ExitView {
                to: e.to.clone(),
                label: e.label.clone(),
                title: story
                    .node(&e.to)
                    .map(|n| n.title.clone())
                    .unwrap_or_default(),
                visited: world.is_visited(&e.to),
            })
            .collect(),
        node: node.clone(),
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontView {
    id: String,
    name: String,
    goal: String,
    steps: Vec<String>,
    progress: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeBrief {
    id: String,
    title: String,
    act: String,
    visited: bool,
    current: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GmRequest {
    #[serde(flatten)]
    request: Request,
    nickname: String,
    character_name: String,
    card_name: Option<String>,
    /// Past rulings for the same situation (pending requests only).
    rulings: Vec<knowledge::Ruling>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LobbySeat {
    player_id: Uuid,
    nickname: String,
    role: players::Role,
    online: bool,
    here: bool,
    sound_ok: bool,
    remote: bool,
    character_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RulesBrief {
    abilities: Vec<(String, String)>,
    difficulties: Vec<(String, String, i32)>,
}

/// `GET /api/campaigns/{id}/session`
///
/// # Errors
///
/// 404 when missing or another GM's; a database error.
pub async fn live_screen(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let pool = &state.pool;
    let rules = row.rules();
    let current = session::current(pool, row.id).await?;
    let last = session::last_ended(pool, row.id).await?;
    let presence = state.live.presence(row.id);
    let seats = players::seats(pool, row.id).await?;
    let attendance = match &current {
        Some(s) => session::attendance(pool, s.id).await?,
        None => Vec::new(),
    };
    let lobby: Vec<LobbySeat> = seats
        .iter()
        .map(|seat| {
            let here = attendance.iter().find(|a| a.player_id == seat.id);
            LobbySeat {
                player_id: seat.id,
                nickname: seat.nickname.clone(),
                role: seat.role,
                online: presence.players.contains(&seat.id),
                here: here.is_some(),
                sound_ok: here.is_some_and(|a| a.sound_ok),
                remote: here.is_none_or(|a| a.remote),
                character_name: seat
                    .character
                    .as_ref()
                    .map(|c| c.name.clone())
                    .filter(|n| !n.is_empty()),
            }
        })
        .collect();
    let mut gm_requests = Vec::new();
    let mut spots = Vec::new();
    if let Some(s) = &current {
        for r in requests::of_session(pool, s.id).await? {
            let seat = seats.iter().find(|x| x.id == r.player_id);
            let situation = if r.text.is_empty() {
                r.card.name(rules).unwrap_or_default()
            } else {
                r.text.clone()
            };
            let rulings = if r.status == requests::RequestStatus::Pending {
                knowledge::similar_rulings(pool, row.id, &situation).await?
            } else {
                Vec::new()
            };
            gm_requests.push(GmRequest {
                nickname: seat.map(|x| x.nickname.clone()).unwrap_or_default(),
                character_name: seat
                    .and_then(|x| x.character.as_ref())
                    .map(|c| c.name.clone())
                    .unwrap_or_default(),
                card_name: r.card.name(rules),
                rulings,
                request: r,
            });
        }
        spots = spotlight::spots(pool, row.id, s, Utc::now()).await?;
    }
    let story = &row.story;
    let world = &row.world;
    // gm/launch-session: « Précédemment… » whole for the GM, how far the
    // table has read, and the scene to send next — where the table
    // stopped, or the campaign's first.
    let launch = match current.as_ref().and_then(|s| s.previously_shown) {
        Some(shown) => session::published_previously(pool, row.id)
            .await?
            .map(|(number, text)| {
                let first = world
                    .current_node
                    .as_deref()
                    .or(story.bible.start_node.as_deref())
                    .and_then(|n| story.node(n))
                    .or_else(|| story.nodes.first());
                json!({
                    "number": number,
                    "lines": recap::lines(&text),
                    "shown": shown,
                    "firstScene": first.map(|n| json!({ "node": n.id, "title": n.title })),
                })
            }),
        None => None,
    };
    let data = json!({
        "session": current,
        "lastEnded": last,
        "launch": launch,
        "presence": presence,
        "lobby": lobby,
        "scene": gm_scene(story, world),
        "nodes": story.nodes.iter().map(|n| NodeBrief {
            id: n.id.clone(),
            title: n.title.clone(),
            act: n.act.clone(),
            visited: world.is_visited(&n.id),
            current: world.current_node.as_deref() == Some(n.id.as_str()),
        }).collect::<Vec<_>>(),
        "startNode": story.bible.start_node,
        "fronts": story.fronts.iter().map(|f| FrontView {
            id: f.id.clone(),
            name: f.name.clone(),
            goal: f.goal.clone(),
            steps: f.steps.iter().map(|s| s.label.clone()).collect(),
            progress: world.front_progress.get(&f.id).copied().unwrap_or(0),
        }).collect::<Vec<_>>(),
        "requests": gm_requests,
        "journal": knowledge::journal(pool, row.id, false).await?,
        "gaps": knowledge::gaps(story, world),
        "spotlight": spots,
        "drafts": match &current {
            Some(s) => drafts::of_session(pool, s.id).await?,
            None => Vec::new(),
        },
        "alertAfterMinutes": spotlight::ALERT_AFTER_MINUTES,
        "rules": rules.map(|r| RulesBrief {
            abilities: r.abilities.iter().map(|a| (a.id.clone(), a.name.clone())).collect(),
            difficulties: r.difficulties.iter().map(|d| (d.id.clone(), d.name.clone(), d.value)).collect(),
        }),
        "ai": { "configured": state.ai.is_configured(), "spending": ledger::spending(pool, row.id).await? },
    });
    Ok(Json(json!({ "data": data })).into_response())
}

fn session_json(s: &Session) -> Response {
    Json(json!({ "data": s })).into_response()
}

/// `POST /api/campaigns/{id}/session`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn open(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let s = session::open(&state.pool, &gm, parse_id(&id)?).await?;
    Ok((StatusCode::CREATED, session_json(&s)).into_response())
}

/// `POST /api/campaigns/{id}/session/start`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`, `SESSION_NOT_IN_LOBBY`.
pub async fn start(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    Ok(session_json(
        &session::start(&state.pool, &gm, parse_id(&id)?).await?,
    ))
}

/// `POST /api/campaigns/{id}/session/end`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`; 400 `TEXT_TOO_LONG`, `INVALID_BODY`.
pub async fn end(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(ending): Body<Ending>,
) -> Result<Response, AppError> {
    Ok(session_json(
        &session::end(&state.pool, &gm, parse_id(&id)?, &ending).await?,
    ))
}

/// `POST /api/campaigns/{id}/session/reveal`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`, `SESSION_NOT_LIVE`; 400 the codes of
/// `evening::scenes::reveal`.
pub async fn reveal(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(reveal): Body<Reveal>,
) -> Result<Response, AppError> {
    scenes::reveal(&state.pool, &gm, parse_id(&id)?, &reveal).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `PUT /api/campaigns/{id}/session/music`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`; 400 `UNKNOWN_TRACK`.
pub async fn music(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(choice): Body<MusicChoice>,
) -> Result<Response, AppError> {
    let music = scenes::music(&state.pool, &gm, parse_id(&id)?, &choice).await?;
    Ok(Json(json!({ "data": music })).into_response())
}

/// `POST /api/campaigns/{id}/session/journal`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`; 400 `EMPTY_TEXT`, `TEXT_TOO_LONG`,
/// `INVALID_KIND`.
pub async fn note(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(note): Body<Note>,
) -> Result<Response, AppError> {
    scenes::note(&state.pool, &gm, parse_id(&id)?, &note).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `POST /api/campaigns/{id}/session/requests/{request}`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`, `SESSION_NOT_LIVE`, `ALREADY_DECIDED`; 400 the
/// codes of `evening::requests::decide`.
pub async fn decide(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, request)): Path<(String, String)>,
    Body(decision): Body<Decision>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let r = requests::decide(
        &state.pool,
        &gm,
        row.id,
        row.rules(),
        parse_id(&request)?,
        &decision,
    )
    .await?;
    Ok(Json(json!({ "data": r })).into_response())
}

/// `POST /api/campaigns/{id}/session/spotlight/{player}`
///
/// # Errors
///
/// 404; 409 `NO_SESSION`, `SESSION_NOT_LIVE`.
pub async fn give_spotlight(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, player)): Path<(String, String)>,
) -> Result<Response, AppError> {
    spotlight::give(&state.pool, &gm, parse_id(&id)?, parse_id(&player)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Played {
    played: bool,
}

/// `PUT /api/campaigns/{id}/hooks/{hook}/played`
///
/// # Errors
///
/// 404 `NO_SUCH_HOOK`.
pub async fn hook_played(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, hook)): Path<(String, String)>,
    Body(body): Body<Played>,
) -> Result<Response, AppError> {
    spotlight::hook_played(
        &state.pool,
        &gm,
        parse_id(&id)?,
        parse_id(&hook)?,
        body.played,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
pub struct KnowledgeQuery {
    #[serde(default, rename = "ref")]
    r#ref: Option<String>,
    #[serde(default)]
    situation: Option<String>,
}

/// `GET /api/campaigns/{id}/knowledge?ref=…&situation=…`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn knowledge(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Query(q): Query<KnowledgeQuery>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let lines = match q.r#ref.as_deref().filter(|r| !r.is_empty()) {
        Some(r) => knowledge::about(&state.pool, row.id, r).await?,
        None => Vec::new(),
    };
    let rulings = match q.situation.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(s) => knowledge::similar_rulings(&state.pool, row.id, s).await?,
        None => knowledge::rulings(&state.pool, row.id).await?,
    };
    Ok(Json(json!({ "data": { "lines": lines, "rulings": rulings } })).into_response())
}

/// `GET /api/campaigns/{id}/sessions`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn chronicle(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let mut entries = Vec::new();
    for s in session::all(&state.pool, row.id).await? {
        let world = world_of(&state, &row, &s).await?;
        let warnings = leaks_of(&row, &world, &s);
        let mut entry = serde_json::to_value(&s).map_err(|e| AppError::internal("session", e))?;
        entry["warnings"] = json!(warnings);
        entries.push(entry);
    }
    Ok(Json(json!({ "data": entries })).into_response())
}

/// `POST /api/campaigns/{id}/session/previously/next` — the next
/// sentence of « Précédemment… » reaches the table (gm/launch-session).
///
/// # Errors
///
/// 404; 409 `NO_SESSION`, `SESSION_NOT_LIVE`, `NOT_READING`.
pub async fn read_next(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    Ok(session_json(
        &session::read_next(&state.pool, &gm, parse_id(&id)?).await?,
    ))
}

/// `PUT /api/campaigns/{id}/sessions/{session}/recap`
///
/// # Errors
///
/// 404; 409 `SESSION_NOT_ENDED`; 400 `TEXT_TOO_LONG`.
pub async fn edit_recap(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, sid)): Path<(String, String)>,
    Body(ending): Body<Ending>,
) -> Result<Response, AppError> {
    let s = session::edit_recap(&state.pool, &gm, parse_id(&id)?, parse_id(&sid)?, &ending).await?;
    Ok(session_json(&s))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecapDraft {
    players: String,
    gm: String,
    #[serde(default)]
    chronicle_title: String,
    #[serde(default)]
    chronicle: String,
}

/// The world `s` ended on — the campaign's world while it runs.
async fn world_of(
    state: &AppState,
    row: &CampaignRow,
    s: &Session,
) -> Result<WorldState, AppError> {
    Ok(session::world_at_end(&state.pool, s.id)
        .await?
        .unwrap_or_else(|| row.world.clone()))
}

/// The names a player text of `s` must not hold, and why — flagged to
/// the GM next to the text, never blocking.
fn leaks_of(row: &CampaignRow, world: &WorldState, s: &Session) -> Vec<recap::Leak> {
    let mut found = recap::leaks(&row.story, world, &s.previously);
    for l in recap::leaks(
        &row.story,
        world,
        &format!("{}\n{}", s.chronicle_title, s.chronicle),
    ) {
        if !found.iter().any(|x| x.name == l.name) {
            found.push(l);
        }
    }
    found
}

/// `POST /api/campaigns/{id}/sessions/{session}/recap-draft` — the
/// co-GM drafts « Précédemment… » and the chronicle entry from what the
/// table saw (the shared journal and the session's facts), and the GM's
/// recap from everything; nothing is saved, the GM edits and publishes.
/// Names the table does not know are flagged in `warnings`.
///
/// # Errors
///
/// 404; 409 `AI_BUDGET_EXCEEDED`; 503 `AI_NOT_CONFIGURED`; 502
/// `AI_UNAVAILABLE`, `AI_OUTPUT_INVALID`.
pub async fn recap_draft(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, sid)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let s = session::by_id(&state.pool, row.id, parse_id(&sid)?).await?;
    let world = world_of(&state, &row, &s).await?;
    let lines: Vec<_> = knowledge::journal(&state.pool, row.id, false)
        .await?
        .into_iter()
        .filter(|l| l.session_id == Some(s.id))
        .collect();
    let shared: Vec<String> = lines
        .iter()
        .filter(|l| l.shared && l.kind != JournalKind::Roll)
        .map(|l| format!("- {}", l.text))
        .collect();
    let mut conn = state.pool.acquire().await?;
    let before = session::world_at_start(&mut conn, &s).await?;
    drop(conn);
    let facts = recap::session_facts(&row.story, &before, &world);
    let told: Vec<String> = facts
        .scenes
        .iter()
        .map(|x| format!("- Scène : {}", x.title))
        .chain(facts.clues.iter().map(|c| format!("- Indice : {}", c.text)))
        .chain(facts.met.iter().map(|m| format!("- Rencontré : {m}")))
        .collect();
    let hidden: Vec<String> = lines
        .iter()
        .filter(|l| !l.shared)
        .map(|l| format!("- {}", l.text))
        .chain(
            facts
                .revelations
                .iter()
                .map(|r| format!("- Désormais à leur portée : {}", r.statement)),
        )
        .chain(facts.fronts.iter().map(|f| {
            format!(
                "- Menace « {} » : {}/{} — {}",
                f.name, f.to, f.total, f.step
            )
        }))
        .chain(
            knowledge::gaps(&row.story, &world)
                .iter()
                .map(|g| format!("- À savoir pour « {} » : {}", g.node_title, g.statement)),
        )
        .collect();
    let secret_names: Vec<String> = row
        .story
        .fronts
        .iter()
        .map(|f| f.name.clone())
        .chain(
            row.story
                .npcs
                .iter()
                .filter(|n| !world.revealed.contains(&n.id))
                .map(|n| n.name.clone()),
        )
        .chain(
            row.story
                .adversaries
                .iter()
                .filter(|a| !world.revealed.contains(&a.id))
                .map(|a| a.name.clone()),
        )
        .map(|n| format!("- {n}"))
        .collect();
    let vars: BTreeMap<&str, String> = [
        ("title", row.story.title.clone()),
        ("number", s.number.to_string()),
        ("journal", shared.join("\n")),
        ("facts", told.join("\n")),
        ("gm", hidden.join("\n")),
        ("secret_names", secret_names.join("\n")),
    ]
    .into_iter()
    .collect();
    let messages = templates::RECAP
        .render(&vars)
        .map_err(|e| AppError::internal("recap template", e))?;
    let answer = state
        .ai
        .complete(
            &state.pool,
            row.id,
            "session.recap",
            &templates::RECAP,
            &LlmRequest::json(messages),
        )
        .await?;
    let draft: RecapDraft = ai::parse_json(&answer.text).map_err(|e| ledger::app_error(&e))?;
    let drafted = Session {
        previously: draft.players.clone(),
        chronicle_title: draft.chronicle_title.clone(),
        chronicle: draft.chronicle.clone(),
        ..s
    };
    Ok(Json(json!({ "data": {
        "players": draft.players,
        "gm": draft.gm,
        "chronicleTitle": draft.chronicle_title,
        "chronicle": draft.chronicle,
        "warnings": leaks_of(&row, &world, &drafted),
    } }))
    .into_response())
}

/// `GET /api/campaigns/{id}/sessions/{session}/feedback`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn feedback_report(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, sid)): Path<(String, String)>,
) -> Result<Response, AppError> {
    let report = feedback::report(&state.pool, &gm, parse_id(&id)?, parse_id(&sid)?).await?;
    Ok(Json(json!({ "data": report })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesBody {
    text: String,
}

/// `PUT /api/campaigns/{id}/sessions/{session}/changes`
///
/// # Errors
///
/// 404; 400 `TEXT_TOO_LONG`.
pub async fn note_changes(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, sid)): Path<(String, String)>,
    Body(body): Body<ChangesBody>,
) -> Result<Response, AppError> {
    feedback::note_changes(
        &state.pool,
        &gm,
        parse_id(&id)?,
        parse_id(&sid)?,
        &body.text,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `GET /api/campaigns/{id}/ai`
///
/// # Errors
///
/// 404 when missing or another GM's.
pub async fn ai_usage(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let row = owned_row(&state, &gm, &id).await?;
    let spending = ledger::spending(&state.pool, row.id).await?;
    let calls = ledger::history(&state.pool, row.id, 50).await?;
    let data: Value = json!({
        "configured": state.ai.is_configured(),
        "spending": spending,
        "leftMicros": spending.left_micros(),
        "calls": calls,
    });
    Ok(Json(json!({ "data": data })).into_response())
}

/// `POST /api/campaigns/{id}/session/copilot` → `{ kind, npc?, prompt }`:
/// a draft of the co-GM (201), for the GM only.
///
/// # Errors
///
/// The codes of `copilot::drafts::ask`.
pub async fn copilot_ask(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path(id): Path<String>,
    Body(ask): Body<copilot::Ask>,
) -> Result<Response, AppError> {
    let d = drafts::ask(&state.pool, &state.ai, &gm, parse_id(&id)?, &ask).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": d }))).into_response())
}

/// `POST /api/campaigns/{id}/session/copilot/{draft}/show` →
/// `{ narration, npcLines: [{ speaker, text }] }`: the GM's edited text
/// reaches the table.
///
/// # Errors
///
/// The codes of `copilot::drafts::show`.
pub async fn copilot_show(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, draft)): Path<(String, String)>,
    Body(show): Body<drafts::Show>,
) -> Result<Response, AppError> {
    drafts::show(&state.pool, &gm, parse_id(&id)?, parse_id(&draft)?, &show).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `POST /api/campaigns/{id}/session/copilot/{draft}/dismiss`
///
/// # Errors
///
/// The codes of `copilot::drafts::dismiss`.
pub async fn copilot_dismiss(
    State(state): State<AppState>,
    gm: CurrentGm,
    Path((id, draft)): Path<(String, String)>,
) -> Result<Response, AppError> {
    drafts::dismiss(&state.pool, &gm, parse_id(&id)?, parse_id(&draft)?).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
