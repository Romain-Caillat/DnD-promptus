//! session/drive-scenes, media/play-youtube-music — what the GM shows.
//!
//! Each gesture is one world write under the campaign lock: the world
//! moves (`world` topic), the journal keeps what the table now knows
//! (`session` topic), and the players' screens refetch their projection.

use chrono::Utc;
use promptus_shared::story::{Campaign, MusicMood, MusicTrack, WorldState};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::knowledge::{self, JournalKind};
use super::session::{self, Music, Status};
use crate::auth::guard::CurrentGm;
use crate::campaigns;
use crate::error::AppError;

/// One gesture of the GM on the story.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Reveal {
    /// Show a scene: it becomes the current one, its first track plays.
    Scene { node: String },
    /// The players find a clue.
    Clue { clue: String },
    /// The players meet an NPC (or learn an adversary's name).
    Npc { npc: String },
    /// A front's clock moves (GM-only: players feel it, they don't see it).
    Front { front: String, delta: i32 },
    /// The scene is resolved.
    Resolve { node: String },
}

/// Apply `reveal` to the live session of `campaign`.
///
/// # Errors
///
/// As `session::lock_for_gm` (the session must be live); 400
/// `UNKNOWN_NODE`, `UNKNOWN_CLUE`, `UNKNOWN_NPC`, `UNKNOWN_FRONT`,
/// `INVALID_DELTA`.
pub async fn reveal(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    reveal: &Reveal,
) -> Result<WorldState, AppError> {
    let mut tx = pool.begin().await?;
    let (mut row, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let story = &row.story;
    let world = &mut row.world;
    let sid = Some(live.id);
    match reveal {
        Reveal::Scene { node } => {
            world
                .enter_node(story, node)
                .map_err(|_| AppError::BadRequest("UNKNOWN_NODE"))?;
            // gm/launch-session: the first scene ends « Précédemment… ».
            session::end_reading(&mut tx, live.id).await?;
            let n = story
                .node(node)
                .ok_or(AppError::BadRequest("UNKNOWN_NODE"))?;
            knowledge::write(
                &mut tx,
                campaign,
                sid,
                JournalKind::Scene,
                Some(node),
                &n.title,
                true,
            )
            .await?;
            let music = first_track(&n.ambience.music).map(|t| Music {
                url: t.url.clone(),
                title: t.title.clone(),
                mood: t.mood,
                started_at: Utc::now(),
            });
            session::set_music(&mut tx, live.id, music.as_ref()).await?;
        }
        Reveal::Clue { clue } => {
            let found = world
                .reveal_clue(story, clue)
                .map_err(|_| AppError::BadRequest("UNKNOWN_CLUE"))?;
            if found != promptus_shared::story::ClueReveal::AlreadyFound {
                let text = story.clue(clue).map(|c| c.text.clone()).unwrap_or_default();
                knowledge::write(
                    &mut tx,
                    campaign,
                    sid,
                    JournalKind::Clue,
                    Some(clue),
                    &text,
                    true,
                )
                .await?;
            }
        }
        Reveal::Npc { npc } => {
            let first = !world.revealed.contains(npc);
            world
                .reveal_entity(story, npc)
                .map_err(|_| AppError::BadRequest("UNKNOWN_NPC"))?;
            if first {
                let text = met_line(story, npc);
                knowledge::write(
                    &mut tx,
                    campaign,
                    sid,
                    JournalKind::Npc,
                    Some(npc),
                    &text,
                    true,
                )
                .await?;
            }
        }
        Reveal::Front { front, delta } => {
            if *delta == 0 || delta.abs() > 6 {
                return Err(AppError::BadRequest("INVALID_DELTA"));
            }
            let moved = world
                .advance_front(story, front, *delta)
                .map_err(|_| AppError::BadRequest("UNKNOWN_FRONT"))?;
            let f = story
                .front(front)
                .ok_or(AppError::BadRequest("UNKNOWN_FRONT"))?;
            let text = match moved.reached.last().and_then(|i| f.steps.get(*i)) {
                Some(step) => format!("{} : {}", f.name, step.label),
                None => format!("{} : {} / {}", f.name, moved.to, f.steps.len()),
            };
            knowledge::write(
                &mut tx,
                campaign,
                sid,
                JournalKind::Note,
                Some(front),
                &text,
                false,
            )
            .await?;
        }
        Reveal::Resolve { node } => {
            world
                .resolve_node(story, node)
                .map_err(|_| AppError::BadRequest("UNKNOWN_NODE"))?;
        }
    }
    let world = row.world.clone();
    campaigns::save_world(&mut tx, campaign, &world).await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(world)
}

/// « Vous avez rencontré Morel, le cartographe » — the name and title,
/// what players may know.
fn met_line(story: &Campaign, id: &str) -> String {
    if let Some(n) = story.npc(id) {
        if n.title.is_empty() {
            return n.name.clone();
        }
        return format!("{}, {}", n.name, n.title);
    }
    story
        .adversary(id)
        .map(|a| a.name.clone())
        .unwrap_or_default()
}

/// The track a scene opens on: the first with a link, calm moods first.
fn first_track(tracks: &[MusicTrack]) -> Option<&MusicTrack> {
    let opening = [
        MusicMood::Calm,
        MusicMood::Exploration,
        MusicMood::Mystery,
        MusicMood::Tension,
        MusicMood::Epic,
        MusicMood::Combat,
    ];
    opening
        .iter()
        .find_map(|mood| tracks.iter().find(|t| t.mood == *mood && !t.url.is_empty()))
}

/// What the GM asks of the music.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MusicChoice {
    /// The index of a track of the current scene; `None` stops the music.
    pub track: Option<usize>,
}

/// Play a track of the current scene for everyone, from its start, or
/// stop the music.
///
/// # Errors
///
/// As `session::lock_for_gm`; 400 `UNKNOWN_TRACK` when the scene has no
/// such track or it has no link yet.
pub async fn music(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    choice: &MusicChoice,
) -> Result<Option<Music>, AppError> {
    let mut tx = pool.begin().await?;
    let (row, live) =
        session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live, Status::Lobby]).await?;
    let music = match choice.track {
        None => None,
        Some(i) => {
            let tracks = row
                .world
                .current_node
                .as_deref()
                .and_then(|n| row.story.node(n))
                .map(|n| n.ambience.music.as_slice())
                .unwrap_or_default();
            let t = tracks
                .get(i)
                .filter(|t| !t.url.is_empty())
                .ok_or(AppError::BadRequest("UNKNOWN_TRACK"))?;
            Some(Music {
                url: t.url.clone(),
                title: t.title.clone(),
                mood: t.mood,
                started_at: Utc::now(),
            })
        }
    };
    session::set_music(&mut tx, live.id, music.as_ref()).await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(music)
}

/// A line the GM writes in the journal: a promise, a debt, a key item,
/// a note; shared with the players or kept.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Note {
    pub kind: JournalKind,
    pub text: String,
    #[serde(default)]
    pub r#ref: Option<String>,
    #[serde(default = "yes")]
    pub shared: bool,
}

fn yes() -> bool {
    true
}

const NOTE_MAX: usize = 1_000;

/// Write `note` in the journal of the open session.
///
/// # Errors
///
/// As `session::lock_for_gm`; 400 `EMPTY_TEXT`, `TEXT_TOO_LONG`,
/// `INVALID_KIND` (the GM writes promises, debts, items, notes and
/// narration; the rest is written by what happens).
pub async fn note(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    note: &Note,
) -> Result<(), AppError> {
    if !matches!(
        note.kind,
        JournalKind::Promise
            | JournalKind::Debt
            | JournalKind::Item
            | JournalKind::Note
            | JournalKind::Narration
    ) {
        return Err(AppError::BadRequest("INVALID_KIND"));
    }
    let text = super::clean_text(&note.text, NOTE_MAX)?;
    if text.is_empty() {
        return Err(AppError::BadRequest("EMPTY_TEXT"));
    }
    let mut tx = pool.begin().await?;
    let (_, live) =
        session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live, Status::Lobby]).await?;
    knowledge::write(
        &mut tx,
        campaign,
        Some(live.id),
        note.kind,
        note.r#ref.as_deref(),
        &text,
        note.shared,
    )
    .await?;
    super::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(())
}
