//! maps/travel-hex-world — the journey on the world map, at the table
//! (migration `044_travel.sql`).
//!
//! The GM shows a world map in hexes (`board::show`): the party is one
//! token there, the fog lifts where it has been. Then, the session live:
//!
//! 1. the GM picks a place; the server proposes the two ways there
//!    (`travel::routes`), which the GM may rename and describe;
//! 2. each player votes on their phone; the GM sees who wants what and
//!    chooses — nothing is decided by the count;
//! 3. the GM plays the day portion by portion: the party moves at each
//!    terrain's pace (`travel::Party::advance`), the supplies are eaten
//!    at dawn;
//! 4. after a portion, three events are drawn from the terrain's table
//!    for the GM only; the GM keeps one (its text may be edited) or none,
//!    and only the one kept reaches the table;
//! 5. a group check: every player rolls on their phone, the server rolls
//!    by the rules and applies the system's group threshold;
//! 6. at night the GM sets the watches and speaks to the one awake only,
//!    who may wake the others;
//! 7. on arrival, entering the place opens its map (the exit on its hex)
//!    and its scene (the label's), without leaving the game screen.
//!
//! The party's hex and the hexes seen live here and are copied onto the
//! board in the same transaction, so a world map comes back as it was
//! left. Every write takes the campaign lock and touches the `map` topic.

use std::collections::{BTreeMap, BTreeSet};

use promptus_shared::maps::{Cell, Map, Scale, Visibility};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::{self, Advantage, OutcomeBand, RollBreakdown, RollTarget};
use promptus_shared::rules::dice::SeededDice;
use promptus_shared::rules::model::{GroupThreshold, RollScope};
use promptus_shared::travel::{
    Advanced, Guide, Party, Route, TravelEvent, Way, draw_events, routes,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::guard::CurrentGm;
use crate::board::{self, Token, TokenKind};
use crate::content;
use crate::error::AppError;
use crate::evening::knowledge::{self, JournalKind};
use crate::evening::session::{self, Status};
use crate::evening::{MomentKind, clean_text, moment};
use crate::live::{self, Topic};
use crate::players::play::{self, Actor, Adjustment, combatant};
use crate::players::{Player, Role};

/// The id of the party's token on a world map.
pub const PARTY_TOKEN: &str = "party";

/// Days of supplies a party starts a world map with.
const START_DAYS: u32 = 3;

/// The longest name, description or line the GM writes here.
const MAX_TEXT: usize = 600;

/// A route as the GM proposes it to the table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposed {
    pub name: String,
    pub description: String,
    pub route: Route,
}

/// Where the journey goes: a place of the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub name: String,
    pub at: Cell,
}

/// An event the GM kept: what the table heard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kept {
    pub day: u32,
    pub portion: String,
    pub event: String,
    pub title: String,
    pub text: String,
}

/// One member's part in a group check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRoll {
    pub character: Uuid,
    pub player: Uuid,
    pub name: String,
    pub roll: Option<RollBreakdown>,
}

/// A group check: everyone rolls, the system's threshold decides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRoll {
    /// What is at stake (« Ne pas se perdre »).
    pub label: String,
    pub ability: String,
    pub ability_name: String,
    pub difficulty: i32,
    pub rolls: Vec<MemberRoll>,
    /// `None` until everyone rolled or the GM closed it.
    pub success: Option<bool>,
    pub successes: usize,
    pub needed: usize,
}

/// One watch of the night.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchSlot {
    pub name: String,
    pub character: Option<Uuid>,
    pub player: Option<Uuid>,
    pub who: Option<String>,
}

/// The night's watches, and what the GM says to the one awake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Watch {
    pub slots: Vec<WatchSlot>,
    /// The watch being played.
    pub current: Option<usize>,
    /// For the watcher of `current` only.
    pub message: String,
    /// The watcher woke the others.
    pub woken: bool,
}

/// The trip under way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Journey {
    pub destination: Destination,
    pub routes: Vec<Proposed>,
    /// Player id → index of the route they want.
    pub votes: BTreeMap<Uuid, usize>,
    pub chosen: Option<usize>,
    pub arrived: bool,
    pub events: Vec<Kept>,
    pub check: Option<GroupRoll>,
    pub watch: Option<Watch>,
}

/// The events drawn for the GM after a portion. GM-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub day: u32,
    pub portion: String,
    pub terrain: String,
    pub events: Vec<TravelEvent>,
}

/// The party on one world map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Travel {
    pub map_id: String,
    pub party: Party,
    pub journey: Option<Journey>,
    pub proposal: Option<Proposal>,
    pub version: i32,
}

type Row = (
    String,
    Json<Party>,
    Option<Json<Journey>>,
    Option<Json<Proposal>>,
    i32,
);

fn from_row((map_id, Json(party), journey, proposal, version): Row) -> Travel {
    Travel {
        map_id,
        party,
        journey: journey.map(|Json(j)| j),
        proposal: proposal.map(|Json(p)| p),
        version,
    }
}

const COLUMNS: &str = "map_id, party, journey, proposal, version";

/// The party on `map_id` of `campaign`, if it has been there.
///
/// # Errors
///
/// A database error.
pub async fn get(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    map_id: &str,
) -> Result<Option<Travel>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM travels WHERE campaign_id = $1 AND map_id = $2"
    ))
    .bind(campaign)
    .bind(map_id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(from_row))
}

/// A member of the party: a validated character and its player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub character: Uuid,
    pub player: Uuid,
    pub name: String,
}

/// The validated characters of `campaign`, in seat order.
///
/// # Errors
///
/// A database error.
pub async fn members(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Vec<Member>, AppError> {
    let rows: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT c.id, p.id, COALESCE(c.sheet->>'name', p.nickname)
         FROM characters c JOIN players p ON p.id = c.player_id
         WHERE p.campaign_id = $1 AND p.role = 'player' AND c.status = 'validated'
         ORDER BY p.created_at",
    )
    .bind(campaign)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(character, player, name)| Member {
            character,
            player,
            name,
        })
        .collect())
}

fn party_size(members: &[Member]) -> u32 {
    u32::try_from(members.len()).unwrap_or(u32::MAX)
}

/// The party's row on `map`, locked, created at the map's party start
/// on first use. `None` when the map has no party start.
async fn locked_or_start(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    map: &Map,
) -> Result<Option<Travel>, AppError> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM travels WHERE campaign_id = $1 AND map_id = $2 FOR UPDATE"
    ))
    .bind(campaign)
    .bind(&map.id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = row {
        return Ok(Some(from_row(row)));
    }
    let guide = content::guide(map);
    let size = party_size(&members(&mut **tx, campaign).await?);
    let supplies = guide.supplies.per_person_per_day * size * START_DAYS;
    let Some(party) = Party::start(map, i32::try_from(supplies).unwrap_or(i32::MAX)) else {
        return Ok(None);
    };
    sqlx::query("INSERT INTO travels (campaign_id, map_id, party) VALUES ($1, $2, $3)")
        .bind(campaign)
        .bind(&map.id)
        .bind(Json(&party))
        .execute(&mut **tx)
        .await?;
    Ok(Some(Travel {
        map_id: map.id.clone(),
        party,
        journey: None,
        proposal: None,
        version: 1,
    }))
}

/// The party's token on a world map.
#[must_use]
pub fn party_token(at: Cell) -> Token {
    Token {
        id: PARTY_TOKEN.into(),
        kind: TokenKind::Character,
        r#ref: PARTY_TOKEN.into(),
        name: "Le groupe".into(),
        at,
        hidden: false,
        invisible: false,
    }
}

/// What the board holds for world map `map` when it is shown: the party
/// token where it stands, and the hexes it has seen. `None` when the map
/// has no party start (the board then shows it bare).
///
/// # Errors
///
/// A database error.
pub(crate) async fn board_parts(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    map: &Map,
) -> Result<Option<(Vec<Token>, BTreeSet<Cell>)>, AppError> {
    Ok(locked_or_start(tx, campaign, map)
        .await?
        .map(|t| (vec![party_token(t.party.at)], t.party.revealed)))
}

/// The GM laid or lifted fog on a world map: the party's memory of it
/// follows, so the map comes back the same.
///
/// # Errors
///
/// A database error.
pub(crate) async fn mirror_fog(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    map_id: &str,
    revealed: &BTreeSet<Cell>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE travels SET party = jsonb_set(party, '{revealed}', $3), version = version + 1,
                updated_at = now()
         WHERE campaign_id = $1 AND map_id = $2",
    )
    .bind(campaign)
    .bind(map_id)
    .bind(Json(revealed))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn save(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    t: &Travel,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE travels SET party = $3, journey = $4, proposal = $5, version = version + 1,
                updated_at = now()
         WHERE campaign_id = $1 AND map_id = $2",
    )
    .bind(campaign)
    .bind(&t.map_id)
    .bind(Json(&t.party))
    .bind(t.journey.as_ref().map(Json))
    .bind(t.proposal.as_ref().map(Json))
    .execute(&mut **tx)
    .await?;
    // The board shows this map: the token and the fog follow the party.
    sqlx::query(
        "UPDATE map_states SET tokens = $3, revealed = $4, updated_at = now()
         WHERE campaign_id = $1 AND map_id = $2",
    )
    .bind(campaign)
    .bind(&t.map_id)
    .bind(Json(vec![party_token(t.party.at)]))
    .bind(Json(&t.party.revealed))
    .execute(&mut **tx)
    .await?;
    live::touch(tx, campaign, &Topic::Map).await?;
    Ok(())
}

/// A place of a world map: a label, the map an exit on its hex opens,
/// and the scene it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub name: String,
    pub at: Cell,
    pub map: Option<String>,
    pub scene: Option<String>,
    /// On a GM layer: the table does not know it yet.
    pub secret: bool,
}

/// The places of `map`, as the GM sees them.
#[must_use]
pub fn places(map: &Map) -> Vec<Place> {
    map.labels
        .iter()
        .map(|l| Place {
            name: l.text.clone(),
            at: l.at,
            map: map
                .exits
                .iter()
                .find(|e| e.cells.contains(&l.at))
                .map(|e| e.to.clone()),
            scene: l.scene.clone(),
            secret: map.visibility(&l.layer) != Visibility::All,
        })
        .collect()
}

/// The world map on the table, and its guide.
async fn world_on_table(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
) -> Result<(Map, Guide), AppError> {
    let b = board::current(&mut **tx, campaign)
        .await?
        .ok_or(AppError::Conflict("NO_WORLD_MAP"))?;
    if b.map.scale != Scale::World {
        return Err(AppError::Conflict("NO_WORLD_MAP"));
    }
    let guide = content::guide(&b.map);
    Ok((b.map, guide))
}

/// One gesture of the GM on the journey.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum GmCommand {
    /// Propose the ways to a place.
    Plan { to: Cell },
    /// Rename or describe a proposed route.
    Route {
        index: usize,
        name: String,
        description: String,
    },
    /// Take a route: the party leaves on it.
    Choose { index: usize },
    /// Play the next portion (or lift the camp). `hold`: the party stays
    /// where it is (lost, resting, a scene that took the time).
    Advance {
        #[serde(default)]
        hold: bool,
    },
    /// Keep one of the events drawn, its text as the GM edited it.
    Keep {
        event: String,
        #[serde(default)]
        text: Option<String>,
    },
    /// Keep none.
    Skip,
    /// A group check: everyone rolls.
    Check {
        ability: String,
        difficulty: i32,
        label: String,
    },
    /// Close the group check with the rolls made.
    CloseCheck,
    /// Who keeps each watch of the night (one entry per watch).
    Watch { slots: Vec<Option<Uuid>> },
    /// Play a watch: the GM's words reach its watcher only.
    WatchTurn { slot: usize, message: String },
    /// Set the supplies.
    Supplies { value: i32 },
    /// Put the party on a hex (no journey under way).
    Place { at: Cell },
    /// Enter the place reached: its map and its scene open.
    Enter,
    /// Drop the journey.
    Abandon,
}

fn journey_mut(t: &mut Travel) -> Result<&mut Journey, AppError> {
    t.journey.as_mut().ok_or(AppError::Conflict("NO_JOURNEY"))
}

fn text(raw: &str) -> Result<String, AppError> {
    clean_text(raw, MAX_TEXT)
}

/// The name a route goes by until the GM renames it: its main terrain.
fn route_name(r: &Route) -> String {
    r.terrains
        .first()
        .map(|(name, _)| capitalised(name))
        .unwrap_or_default()
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Apply `cmd` to the journey on the world map at `campaign`'s table.
///
/// # Errors
///
/// As `session::lock_for_gm` (the session must be live); 409
/// `NO_WORLD_MAP`, `NO_PARTY_START`, `NO_JOURNEY`, `JOURNEY_UNDER_WAY`,
/// `NOT_CHOSEN`, `ALREADY_ARRIVED`, `NOT_ARRIVED`, `NO_PROPOSAL`,
/// `NO_CHECK`, `CHECK_OPEN`, `NOT_NIGHT`, `NO_WATCH`, `NO_MEMBERS`,
/// `RULES_UNKNOWN`, `FIGHT_IN_PROGRESS`; 400 `CELL_OUTSIDE`,
/// `NO_ROUTE`, `UNKNOWN_ROUTE`, `UNKNOWN_EVENT`, `UNKNOWN_ABILITY`,
/// `INVALID_DIFFICULTY`, `INVALID_WATCH`, `NOT_A_MEMBER`,
/// `INVALID_SUPPLIES`, `CANNOT_STAND`, `NO_PLACE_MAP`, `TEXT_TOO_LONG`.
pub async fn gm(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    cmd: &GmCommand,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let (mut row, live) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let (map, guide) = world_on_table(&mut tx, campaign).await?;
    let mut t = locked_or_start(&mut tx, campaign, &map)
        .await?
        .ok_or(AppError::Conflict("NO_PARTY_START"))?;
    let sid = Some(live.id);
    match cmd {
        GmCommand::Plan { to } => {
            if t.journey
                .as_ref()
                .is_some_and(|j| j.chosen.is_some() && !j.arrived)
            {
                return Err(AppError::Conflict("JOURNEY_UNDER_WAY"));
            }
            if !map.grid.contains(*to) {
                return Err(AppError::BadRequest("CELL_OUTSIDE"));
            }
            let found = routes(&map, &guide, t.party.at, *to);
            if found.is_empty() {
                return Err(AppError::BadRequest("NO_ROUTE"));
            }
            let name = map
                .labels
                .iter()
                .find(|l| l.at == *to)
                .map_or_else(String::new, |l| l.text.clone());
            t.journey = Some(Journey {
                destination: Destination { name, at: *to },
                routes: found
                    .into_iter()
                    .map(|r| Proposed {
                        name: route_name(&r),
                        description: String::new(),
                        route: r,
                    })
                    .collect(),
                votes: BTreeMap::new(),
                chosen: None,
                arrived: false,
                events: Vec::new(),
                check: None,
                watch: None,
            });
            t.proposal = None;
            t.party.way = None;
        }
        GmCommand::Route {
            index,
            name,
            description,
        } => {
            let j = journey_mut(&mut t)?;
            let r = j
                .routes
                .get_mut(*index)
                .ok_or(AppError::BadRequest("UNKNOWN_ROUTE"))?;
            r.name = text(name)?;
            r.description = text(description)?;
        }
        GmCommand::Choose { index } => {
            let j = journey_mut(&mut t)?;
            if j.chosen.is_some() {
                return Err(AppError::Conflict("JOURNEY_UNDER_WAY"));
            }
            let r = j
                .routes
                .get(*index)
                .ok_or(AppError::BadRequest("UNKNOWN_ROUTE"))?;
            let line = format!("En route vers {} : {}.", j.destination.name, r.name);
            let hexes = r.route.hexes.clone();
            j.chosen = Some(*index);
            t.party.way = Some(Way {
                hexes,
                step: 0,
                bank: 0,
            });
            knowledge::write(
                &mut tx,
                campaign,
                sid,
                JournalKind::Narration,
                None,
                &line,
                true,
            )
            .await?;
        }
        GmCommand::Advance { hold } => {
            if board::fight::live_id(&mut *tx, campaign).await?.is_some() {
                return Err(AppError::Conflict("FIGHT_IN_PROGRESS"));
            }
            if t.journey.as_ref().is_some_and(|j| j.arrived) {
                return Err(AppError::Conflict("ALREADY_ARRIVED"));
            }
            if t.journey.as_ref().is_some_and(|j| j.chosen.is_none()) {
                return Err(AppError::Conflict("NOT_CHOSEN"));
            }
            let size = party_size(&members(&mut *tx, campaign).await?);
            let advanced = t.party.advance(&guide, &map, *hold, size);
            let line = advanced_line(&guide, &map, &t.party, &advanced);
            t.proposal = None;
            if let Advanced::Portion {
                day,
                name,
                arrived,
                night,
                ..
            } = &advanced
            {
                if let Some(j) = t.journey.as_mut() {
                    j.arrived = *arrived;
                    // A new watch each night.
                    if *night {
                        j.watch = None;
                    }
                }
                if !arrived {
                    let spent: Vec<String> = t
                        .journey
                        .as_ref()
                        .map(|j| j.events.iter().map(|e| e.event.clone()).collect())
                        .unwrap_or_default();
                    let drawn =
                        draw_events(&guide, &map, t.party.at, &spent, &mut SeededDice::from_os());
                    if !drawn.is_empty() {
                        t.proposal = Some(Proposal {
                            day: *day,
                            portion: name.clone(),
                            terrain: guide.terrain_name(&map, t.party.at),
                            events: drawn.into_iter().cloned().collect(),
                        });
                    }
                }
            }
            if matches!(advanced, Advanced::Dawn { .. })
                && let Some(j) = t.journey.as_mut()
            {
                j.watch = None;
            }
            knowledge::write(
                &mut tx,
                campaign,
                sid,
                JournalKind::Narration,
                None,
                &line,
                true,
            )
            .await?;
        }
        GmCommand::Keep {
            event,
            text: edited,
        } => {
            let p = t.proposal.take().ok_or(AppError::Conflict("NO_PROPOSAL"))?;
            let e = p
                .events
                .iter()
                .find(|e| &e.id == event)
                .ok_or(AppError::BadRequest("UNKNOWN_EVENT"))?;
            let said = match edited.as_deref().map(text).transpose()? {
                Some(s) if !s.is_empty() => s,
                _ => e.text.clone(),
            };
            let kept = Kept {
                day: p.day,
                portion: p.portion.clone(),
                event: e.id.clone(),
                title: e.title.clone(),
                text: said,
            };
            knowledge::write(
                &mut tx,
                campaign,
                sid,
                JournalKind::Narration,
                None,
                &format!("{} — {}", kept.title, kept.text),
                true,
            )
            .await?;
            if !e.gm_notes.is_empty() {
                knowledge::write(
                    &mut tx,
                    campaign,
                    sid,
                    JournalKind::Note,
                    None,
                    &format!("{} : {}", kept.title, e.gm_notes),
                    false,
                )
                .await?;
            }
            journey_mut(&mut t)?.events.push(kept);
        }
        GmCommand::Skip => {
            t.proposal.take().ok_or(AppError::Conflict("NO_PROPOSAL"))?;
        }
        GmCommand::Check {
            ability,
            difficulty,
            label,
        } => {
            let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
            let ability_name = rules
                .abilities
                .iter()
                .find(|a| &a.id == ability)
                .map(|a| a.name.clone())
                .ok_or(AppError::BadRequest("UNKNOWN_ABILITY"))?;
            if !(1..=40).contains(difficulty) {
                return Err(AppError::BadRequest("INVALID_DIFFICULTY"));
            }
            let label = text(label)?;
            let all = members(&mut *tx, campaign).await?;
            if all.is_empty() {
                return Err(AppError::Conflict("NO_MEMBERS"));
            }
            let j = journey_mut(&mut t)?;
            if j.check.as_ref().is_some_and(|c| c.success.is_none()) {
                return Err(AppError::Conflict("CHECK_OPEN"));
            }
            j.check = Some(GroupRoll {
                label,
                ability: ability.clone(),
                ability_name,
                difficulty: *difficulty,
                needed: threshold(rules).needed(all.len()),
                rolls: all
                    .into_iter()
                    .map(|m| MemberRoll {
                        character: m.character,
                        player: m.player,
                        name: m.name,
                        roll: None,
                    })
                    .collect(),
                success: None,
                successes: 0,
            });
        }
        GmCommand::CloseCheck => {
            let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
            let j = journey_mut(&mut t)?;
            let c = j
                .check
                .as_mut()
                .filter(|c| c.success.is_none())
                .ok_or(AppError::Conflict("NO_CHECK"))?;
            // Who did not roll is out of the count.
            c.rolls.retain(|r| r.roll.is_some());
            c.needed = threshold(rules).needed(c.rolls.len());
            let line = settle(c);
            knowledge::write(&mut tx, campaign, sid, JournalKind::Roll, None, &line, true).await?;
        }
        GmCommand::Watch { slots } => {
            if !t.party.clock.night {
                return Err(AppError::Conflict("NOT_NIGHT"));
            }
            if slots.len() != guide.watches.len() {
                return Err(AppError::BadRequest("INVALID_WATCH"));
            }
            let all = members(&mut *tx, campaign).await?;
            let mut out = Vec::new();
            for (name, who) in guide.watches.iter().zip(slots) {
                let m = match who {
                    Some(c) => Some(
                        all.iter()
                            .find(|m| m.character == *c)
                            .ok_or(AppError::BadRequest("NOT_A_MEMBER"))?,
                    ),
                    None => None,
                };
                out.push(WatchSlot {
                    name: name.clone(),
                    character: m.map(|m| m.character),
                    player: m.map(|m| m.player),
                    who: m.map(|m| m.name.clone()),
                });
            }
            journey_mut(&mut t)?.watch = Some(Watch {
                slots: out,
                current: None,
                message: String::new(),
                woken: false,
            });
        }
        GmCommand::WatchTurn { slot, message } => {
            let message = text(message)?;
            let j = journey_mut(&mut t)?;
            let w = j.watch.as_mut().ok_or(AppError::Conflict("NO_WATCH"))?;
            if *slot >= w.slots.len() {
                return Err(AppError::BadRequest("INVALID_WATCH"));
            }
            w.current = Some(*slot);
            w.message = message;
            w.woken = false;
        }
        GmCommand::Supplies { value } => {
            if !(-999..=9999).contains(value) {
                return Err(AppError::BadRequest("INVALID_SUPPLIES"));
            }
            t.party.supplies = *value;
        }
        GmCommand::Place { at } => {
            if t.journey
                .as_ref()
                .is_some_and(|j| j.chosen.is_some() && !j.arrived)
            {
                return Err(AppError::Conflict("JOURNEY_UNDER_WAY"));
            }
            if !map.grid.contains(*at) {
                return Err(AppError::BadRequest("CELL_OUTSIDE"));
            }
            if guide.cost(&map, *at).is_none() {
                return Err(AppError::BadRequest("CANNOT_STAND"));
            }
            t.party.place(&map, *at);
            t.journey = None;
            t.proposal = None;
        }
        GmCommand::Enter => {
            let j = t.journey.take().ok_or(AppError::Conflict("NO_JOURNEY"))?;
            if !j.arrived {
                return Err(AppError::Conflict("NOT_ARRIVED"));
            }
            t.proposal = None;
            t.party.way = None;
            let place = places(&map).into_iter().find(|p| p.at == j.destination.at);
            let target = place
                .as_ref()
                .and_then(|p| p.map.clone())
                .ok_or(AppError::BadRequest("NO_PLACE_MAP"))?;
            // The party's row first: showing the place map replaces the
            // board, whose copy of the world map then goes.
            save(&mut tx, campaign, &t).await?;
            knowledge::write(
                &mut tx,
                campaign,
                sid,
                JournalKind::Narration,
                None,
                &format!(
                    "Arrivée : {} (jour {}).",
                    j.destination.name, t.party.clock.day
                ),
                true,
            )
            .await?;
            if let Some(node) = place.as_ref().and_then(|p| p.scene.as_deref()) {
                crate::evening::scenes::enter(&mut tx, &mut row, live.id, node).await?;
            }
            board::show_in(&mut tx, &row, &target).await?;
            crate::evening::touch(&mut tx, campaign).await?;
            tx.commit().await?;
            return Ok(());
        }
        GmCommand::Abandon => {
            t.journey.take().ok_or(AppError::Conflict("NO_JOURNEY"))?;
            t.proposal = None;
            t.party.way = None;
        }
    }
    save(&mut tx, campaign, &t).await?;
    crate::evening::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(())
}

/// The group threshold of the rules; at least half when they say none.
fn threshold(rules: &RuleSystem) -> GroupThreshold {
    rules
        .group_check
        .as_ref()
        .map_or(GroupThreshold::AtLeastHalf, |g| g.succeeds_when)
}

/// Close `c` and say how it went.
fn settle(c: &mut GroupRoll) -> String {
    c.successes = c
        .rolls
        .iter()
        .filter(|r| {
            r.roll
                .as_ref()
                .is_some_and(|b| b.band.is_some_and(OutcomeBand::is_success))
        })
        .count();
    let ok = !c.rolls.is_empty() && c.successes >= c.needed;
    c.success = Some(ok);
    format!(
        "Jet de groupe — {} ({} {}) : {} sur {}, {}.",
        c.label,
        c.ability_name,
        c.difficulty,
        c.successes,
        c.rolls.len(),
        if ok { "réussi" } else { "raté" }
    )
}

/// The journal line of a gesture of time.
fn advanced_line(guide: &Guide, map: &Map, party: &Party, a: &Advanced) -> String {
    match a {
        Advanced::Portion {
            day,
            name,
            entered,
            arrived,
            ..
        } => {
            let terrain = guide.terrain_name(map, party.at);
            let moved = match (entered.len(), arrived) {
                (_, true) => "le groupe arrive".to_string(),
                (0, _) => "le groupe ne bouge pas".to_string(),
                (1, _) => format!("un hexagone, {terrain}"),
                (n, _) => format!("{n} hexagones, {terrain}"),
            };
            format!("Jour {day} · {name} : {moved}.")
        }
        Advanced::Dawn { day, eaten, hungry } => {
            let s = &guide.supplies.name;
            if *hungry {
                format!("Jour {day} · le camp est levé : {s} épuisés, le groupe a faim.")
            } else {
                format!("Jour {day} · le camp est levé : {eaten} {s} consommés.")
            }
        }
    }
}

/// What a player does on the journey.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlayerCommand {
    /// Vote for a proposed route (again to change one's mind).
    Vote { route: usize },
    /// Roll one's part of the group check.
    Roll,
    /// The watcher wakes the others.
    Wake,
}

/// Apply `cmd` for `player`.
///
/// # Errors
///
/// 403 `SPECTATOR`; as `session::lock_for_player`; 409 `NO_WORLD_MAP`,
/// `NO_JOURNEY`, `VOTE_CLOSED`, `NO_CHECK`, `ALREADY_ROLLED`,
/// `NOT_IN_CHECK`, `NOT_YOUR_WATCH`, `RULES_UNKNOWN`, `NO_PLAY_SHEET`;
/// 400 `UNKNOWN_ROUTE`.
pub async fn player(pool: &PgPool, player: &Player, cmd: &PlayerCommand) -> Result<(), AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let campaign = player.campaign_id;
    let mut tx = pool.begin().await?;
    let (row, live) = session::lock_for_player(&mut tx, player, &[Status::Live]).await?;
    let (map, _) = world_on_table(&mut tx, campaign).await?;
    let mut t = locked_or_start(&mut tx, campaign, &map)
        .await?
        .ok_or(AppError::Conflict("NO_JOURNEY"))?;
    let sid = Some(live.id);
    match cmd {
        PlayerCommand::Vote { route } => {
            let j = journey_mut(&mut t)?;
            if j.chosen.is_some() {
                return Err(AppError::Conflict("VOTE_CLOSED"));
            }
            if *route >= j.routes.len() {
                return Err(AppError::BadRequest("UNKNOWN_ROUTE"));
            }
            j.votes.insert(player.id, *route);
        }
        PlayerCommand::Roll => {
            let rules = row.rules().ok_or(AppError::Conflict("RULES_UNKNOWN"))?;
            let (character, ability, difficulty) = {
                let c = journey_mut(&mut t)?
                    .check
                    .as_ref()
                    .filter(|c| c.success.is_none())
                    .ok_or(AppError::Conflict("NO_CHECK"))?;
                let mine = c
                    .rolls
                    .iter()
                    .find(|r| r.player == player.id)
                    .ok_or(AppError::Conflict("NOT_IN_CHECK"))?;
                if mine.roll.is_some() {
                    return Err(AppError::Conflict("ALREADY_ROLLED"));
                }
                (mine.character, c.ability.clone(), c.difficulty)
            };
            let (sheet, state) = play::in_play_locked(&mut tx, campaign, character, rules).await?;
            let fighter =
                combatant(rules, &sheet, &state).ok_or(AppError::Conflict("NO_PLAY_SHEET"))?;
            let target = RollTarget::Difficulty {
                id: rules
                    .difficulties
                    .iter()
                    .find(|d| d.value == difficulty)
                    .map(|d| d.id.clone()),
                value: difficulty,
            };
            let breakdown = check::ability_check(
                rules,
                &fighter,
                &ability,
                RollScope::Checks,
                Some(target),
                Advantage::Normal,
                &mut SeededDice::from_os(),
            )
            .map_err(|e| AppError::Internal(format!("group check: {e:?}")))?;
            let xp = breakdown.band.map_or(0, |b| b.xp(rules));
            if xp > 0 {
                play::adjust_in(
                    &mut tx,
                    campaign,
                    rules,
                    character,
                    Adjustment::Xp {
                        delta: i32::try_from(xp).unwrap_or(i32::MAX),
                    },
                    Actor::Rules,
                )
                .await?;
            }
            let c = journey_mut(&mut t)?
                .check
                .as_mut()
                .ok_or(AppError::Conflict("NO_CHECK"))?;
            if let Some(r) = c.rolls.iter_mut().find(|r| r.player == player.id) {
                r.roll = Some(breakdown);
            }
            if c.rolls.iter().all(|r| r.roll.is_some()) {
                let line = settle(c);
                knowledge::write(&mut tx, campaign, sid, JournalKind::Roll, None, &line, true)
                    .await?;
            }
            moment(&mut tx, live.id, player.id, MomentKind::Roll).await?;
        }
        PlayerCommand::Wake => {
            let j = journey_mut(&mut t)?;
            let w = j
                .watch
                .as_mut()
                .ok_or(AppError::Conflict("NOT_YOUR_WATCH"))?;
            let who = w
                .current
                .and_then(|i| w.slots.get(i))
                .filter(|s| s.player == Some(player.id))
                .and_then(|s| s.who.clone())
                .ok_or(AppError::Conflict("NOT_YOUR_WATCH"))?;
            w.woken = true;
            knowledge::write(
                &mut tx,
                campaign,
                sid,
                JournalKind::Narration,
                None,
                &format!("{who} réveille le groupe."),
                true,
            )
            .await?;
            moment(&mut tx, live.id, player.id, MomentKind::Request).await?;
        }
    }
    save(&mut tx, campaign, &t).await?;
    crate::evening::touch(&mut tx, campaign).await?;
    tx.commit().await?;
    Ok(())
}

/// The world map on `campaign`'s table, its guide and the party there,
/// for the views. `None` when no world map is shown.
///
/// # Errors
///
/// A database error.
pub async fn on_table(
    pool: &PgPool,
    campaign: Uuid,
) -> Result<Option<(Map, Guide, Option<Travel>)>, AppError> {
    let Some(b) = board::current(pool, campaign).await? else {
        return Ok(None);
    };
    if b.map.scale != Scale::World {
        return Ok(None);
    }
    let t = get(pool, campaign, &b.map_id).await?;
    let guide = content::guide(&b.map);
    Ok(Some((b.map, guide, t)))
}
