//! The grid at the table (migration `013_board.sql`): the map the GM
//! shows, its fog, the tokens out of a fight, and the fight itself
//! ([`fight`]).
//!
//! - player/explore-map: a player drags their own token; the server
//!   checks the path cell by cell (`maps::check_path`) and lifts the fog
//!   from where the party now sees;
//! - maps/reveal-fog-and-hidden: the GM lifts or lays the fog, reveals a
//!   hidden layer or one object, opens a door, hides a token or makes it
//!   invisible; players receive the map cut by `Map::project` then
//!   `Map::fogged`, and no token on a fogged cell or hidden from them
//!   (`campaigns::projection::board`);
//! - maps/blend-outdoor-terrain: the GM changes the time, the weather or
//!   the light live;
//! - characters/walk-in-four-directions: each token carries its look, the
//!   way it faces and its last move (`trail`, numbered by `moves`), so
//!   every screen shows the character walk that path and turn toward
//!   where it went or what it acted on;
//! - maps/travel-hex-world: on a world map in hexes the party is one
//!   token, moved only by the journey (`crate::travel`); nobody drags
//!   it, and the fog lifts around the hexes it entered, not by sight.
//!
//! Every write takes the campaign lock and touches the `map` topic.

pub mod fight;
pub mod rewards;

use std::collections::{BTreeMap, BTreeSet};

use promptus_shared::maps::{
    Cell, DoorState, Geometry, LightLevel, Map, MovementRules, Occupancy, Scale, Side as MapSide,
    TimeOfDay, Visibility, Weather, check_path, neighbours, reachable, visible_cells,
};
use promptus_shared::rules::RuleSystem;
use promptus_shared::sprite::{CharacterLook, Direction};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::guard::CurrentGm;
use crate::campaigns::{self, CampaignRow};
use crate::content;
use crate::error::AppError;
use crate::evening::session::{self, Status};
use crate::live::{self, Topic};
use crate::players::{self, CharacterStatus, Player, Role};

/// How far a token walks in one drag out of a fight, in movement
/// points: two moves of the fight's default (INTERPRETATION: exploring
/// is not timed; the limit only keeps one drag on screen).
pub const EXPLORE_BUDGET: u32 = 2 * promptus_shared::combat::fight::DEFAULT_CELLS_PER_MOVE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    /// A player's character: `ref` is the character's id.
    Character,
    /// An NPC or adversary of the story: `ref` is its id.
    Npc,
}

/// Someone standing on the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub id: String,
    pub kind: TokenKind,
    pub r#ref: String,
    pub name: String,
    pub at: Cell,
    /// The GM keeps it to themselves (an ambusher not yet seen).
    #[serde(default)]
    pub hidden: bool,
    /// Invisible: gone from every screen but its owner's, where it is a
    /// ghost.
    #[serde(default)]
    pub invisible: bool,
    /// What its sprite draws, set when it is put on the map: the
    /// character's sheet, or the NPC's look (`content::npc_look`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<CharacterLook>,
    /// Where it faces; none until it first moves or acts (see
    /// `Token::facing`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facing: Option<Direction>,
    /// Its last move: the cell it left, then every cell it entered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trail: Vec<Cell>,
    /// How many moves it made: a screen walks a trail it has not shown
    /// yet, and only that one.
    #[serde(default)]
    pub moves: u32,
}

impl Token {
    /// A token as it is first put on the map, facing its side's way.
    #[must_use]
    pub fn new(
        id: String,
        kind: TokenKind,
        r#ref: String,
        name: String,
        at: Cell,
        look: Option<CharacterLook>,
    ) -> Self {
        Self {
            id,
            kind,
            r#ref,
            name,
            at,
            hidden: false,
            invisible: false,
            look,
            facing: None,
            trail: Vec::new(),
            moves: 0,
        }
    }

    /// Where it faces: the way it last turned, else its side's rest —
    /// heroes east, foes west (`MEMORY.md` §2).
    #[must_use]
    pub fn facing(&self) -> Direction {
        self.facing.unwrap_or(match self.kind {
            TokenKind::Character => Direction::East,
            TokenKind::Npc => Direction::West,
        })
    }

    /// Walk along `path`, the cells entered in order (already checked):
    /// it ends on the last one, facing its last step.
    pub fn walk(&mut self, path: &[Cell]) {
        let Some(&to) = path.last() else { return };
        let mut trail = Vec::with_capacity(path.len() + 1);
        trail.push(self.at);
        trail.extend_from_slice(path);
        if let Some(d) = Direction::toward(trail[trail.len() - 2], to) {
            self.facing = Some(d);
        }
        self.at = to;
        self.trail = trail;
        self.moves += 1;
    }

    /// Turn toward `cell` (a target), staying where it stands.
    pub fn face(&mut self, cell: Cell) {
        if let Some(d) = Direction::toward(self.at, cell) {
            self.facing = Some(d);
        }
    }
}

/// The map at the table, as the server holds it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub map_id: String,
    /// The map as it stands: doors opened, layers revealed, ambience
    /// changed by the GM.
    pub map: Map,
    pub fog: bool,
    pub revealed: BTreeSet<Cell>,
    pub tokens: Vec<Token>,
}

type Row = (
    String,
    Json<Map>,
    bool,
    Json<BTreeSet<Cell>>,
    Json<Vec<Token>>,
);

fn board((map_id, Json(map), fog, Json(revealed), Json(tokens)): Row) -> Board {
    Board {
        map_id,
        map,
        fog,
        revealed,
        tokens,
    }
}

/// The map shown at `campaign`'s table, if any.
///
/// # Errors
///
/// A database error.
pub async fn current(db: impl PgExecutor<'_>, campaign: Uuid) -> Result<Option<Board>, AppError> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT map_id, map, fog, revealed, tokens FROM map_states WHERE campaign_id = $1",
    )
    .bind(campaign)
    .fetch_optional(db)
    .await?;
    Ok(row.map(board))
}

pub(crate) async fn save(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    b: &Board,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO map_states (campaign_id, map_id, map, fog, revealed, tokens)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (campaign_id) DO UPDATE
           SET map_id = EXCLUDED.map_id, map = EXCLUDED.map, fog = EXCLUDED.fog,
               revealed = EXCLUDED.revealed, tokens = EXCLUDED.tokens, updated_at = now()",
    )
    .bind(campaign)
    .bind(&b.map_id)
    .bind(Json(&b.map))
    .bind(b.fog)
    .bind(Json(&b.revealed))
    .bind(Json(&b.tokens))
    .execute(&mut **tx)
    .await?;
    live::touch(tx, campaign, &Topic::Map).await?;
    Ok(())
}

/// The movement costs of the campaign's rules on `map`.
#[must_use]
pub fn movement(rules: Option<&RuleSystem>, map: &Map) -> MovementRules {
    rules.map_or_else(MovementRules::default, |r| {
        promptus_shared::combat::fight::movement_rules(r, map)
    })
}

impl Board {
    /// Lift the fog from every cell a character token sees — on a world
    /// map, from the hexes around the party.
    pub fn reveal_from_party(&mut self) {
        let limit = self.map.ambience.sight_limit;
        let from: Vec<Cell> = self
            .tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Character)
            .map(|t| t.at)
            .collect();
        for at in from {
            if self.map.geometry() == Geometry::Hex {
                self.revealed.insert(at);
                let around = neighbours(Geometry::Hex, at);
                let grid = &self.map.grid;
                self.revealed
                    .extend(around.into_iter().filter(|c| grid.contains(*c)));
            } else {
                self.revealed.extend(visible_cells(&self.map, at, limit));
            }
        }
    }

    fn occupancy_for(&self, token: &str) -> Occupancy {
        let mut o = Occupancy::default();
        for t in self.tokens.iter().filter(|t| t.id != token) {
            match t.kind {
                TokenKind::Character => o.allies.insert(t.at),
                TokenKind::Npc => o.enemies.insert(t.at),
            };
        }
        o
    }

    /// Where `token` may walk in one drag, with the cost of each cell.
    #[must_use]
    pub fn reachable(&self, rules: Option<&RuleSystem>, token: &str) -> BTreeMap<Cell, u32> {
        let Some(t) = self.tokens.iter().find(|t| t.id == token) else {
            return BTreeMap::new();
        };
        reachable(
            &self.map,
            &movement(rules, &self.map),
            &self.occupancy_for(token),
            t.at,
            EXPLORE_BUDGET,
        )
    }
}

/// The token id of a character.
#[must_use]
pub fn character_token(character: Uuid) -> String {
    format!("pc-{character}")
}

/// Put the validated characters of `campaign` on the party starts of
/// `map` (those already on the board keep their cell when it exists).
async fn party_tokens(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    map: &Map,
    before: &[Token],
) -> Result<Vec<Token>, AppError> {
    let rows: Vec<(Uuid, String, Option<Json<CharacterLook>>)> = sqlx::query_as(
        "SELECT c.id, COALESCE(c.sheet->>'name', p.nickname), c.sheet->'look'
         FROM characters c JOIN players p ON p.id = c.player_id
         WHERE p.campaign_id = $1 AND p.role = 'player' AND c.status = 'validated'
         ORDER BY p.created_at",
    )
    .bind(campaign)
    .fetch_all(&mut **tx)
    .await?;
    let mut starts = map
        .starts
        .iter()
        .filter(|s| s.side == Some(MapSide::Party))
        .map(|s| s.at);
    let mut tokens = Vec::new();
    for (id, name, look) in rows {
        let tid = character_token(id);
        let kept = before
            .iter()
            .find(|t| t.id == tid)
            .filter(|t| map.grid.contains(t.at));
        let Some(at) = kept.map(|t| t.at).or_else(|| starts.next()) else {
            break;
        };
        let mut token = Token::new(
            tid,
            TokenKind::Character,
            id.to_string(),
            name,
            at,
            look.map(|Json(l)| l),
        );
        // A token that stays keeps the way it faced and its last move.
        if let Some(k) = kept {
            token.facing = k.facing;
            token.trail.clone_from(&k.trail);
            token.moves = k.moves;
        }
        tokens.push(token);
    }
    Ok(tokens)
}

/// What the GM can show: the campaign's validated maps, then its
/// world's.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapChoice {
    pub id: String,
    pub name: String,
    /// The scenes that name it.
    pub nodes: Vec<String>,
}

/// # Errors
///
/// A database error.
pub async fn choices(
    db: impl PgExecutor<'_>,
    row: &CampaignRow,
) -> Result<Vec<MapChoice>, AppError> {
    let own = crate::campaign_maps::validated(db, row.id).await?;
    let world: Vec<&Map> = content::maps(&row.story)
        .filter(|m| !own.iter().any(|o| o.id == m.id))
        .collect();
    let choices = own
        .iter()
        .chain(world)
        .map(|m| MapChoice {
            id: m.id.clone(),
            name: m.name.clone(),
            nodes: row
                .story
                .nodes
                .iter()
                .filter(|n| n.map.as_deref() == Some(m.id.as_str()))
                .map(|n| n.id.clone())
                .collect(),
        })
        .collect();
    Ok(choices)
}

/// Show map `map_id` at the table: the party on its starts, the fog
/// lifted from where they stand. Showing the map already shown keeps its
/// state. The session must be live.
///
/// # Errors
///
/// 404; 409 `NO_SESSION`, `SESSION_NOT_LIVE`, `FIGHT_IN_PROGRESS`; 400
/// `UNKNOWN_MAP`.
pub async fn show(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    map_id: &str,
) -> Result<Board, AppError> {
    let mut tx = pool.begin().await?;
    let (row, _) = session::lock_for_gm(&mut tx, gm, campaign, &[Status::Live]).await?;
    let b = show_in(&mut tx, &row, map_id).await?;
    tx.commit().await?;
    Ok(b)
}

/// [`show`] inside the caller's transaction, the campaign lock held and
/// the session checked live (maps/travel-hex-world enters a place this
/// way). A world map in hexes comes back with the party where it stands
/// and the hexes it has seen (`crate::travel`).
///
/// # Errors
///
/// 409 `FIGHT_IN_PROGRESS`; 400 `UNKNOWN_MAP`.
pub(crate) async fn show_in(
    tx: &mut Transaction<'_, Postgres>,
    row: &CampaignRow,
    map_id: &str,
) -> Result<Board, AppError> {
    let campaign = row.id;
    if fight::live_id(&mut **tx, campaign).await?.is_some() {
        return Err(AppError::Conflict("FIGHT_IN_PROGRESS"));
    }
    let before = current(&mut **tx, campaign).await?;
    if let Some(b) = before.as_ref().filter(|b| b.map_id == map_id) {
        return Ok(b.clone());
    }
    let map = crate::campaign_maps::playable(&mut **tx, campaign, &row.story, map_id)
        .await?
        .ok_or(AppError::BadRequest("UNKNOWN_MAP"))?;
    let (tokens, revealed) = if map.scale == Scale::World {
        crate::travel::board_parts(tx, campaign, &map)
            .await?
            .unwrap_or_default()
    } else {
        (
            party_tokens(tx, campaign, &map, &[]).await?,
            BTreeSet::new(),
        )
    };
    let mut b = Board {
        map_id: map_id.to_string(),
        map,
        fog: true,
        revealed,
        tokens,
    };
    b.reveal_from_party();
    save(tx, campaign, &b).await?;
    Ok(b)
}

/// Lock the board of `campaign` (under the campaign lock already held).
async fn locked(tx: &mut Transaction<'_, Postgres>, campaign: Uuid) -> Result<Board, AppError> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT map_id, map, fog, revealed, tokens FROM map_states WHERE campaign_id = $1 FOR UPDATE",
    )
    .bind(campaign)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(board).ok_or(AppError::Conflict("NO_MAP_SHOWN"))
}

/// One gesture of the GM on the map.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Edit {
    /// Lift the fog from these cells.
    RevealCells {
        cells: Vec<Cell>,
    },
    /// Lay the fog back on these cells.
    HideCells {
        cells: Vec<Cell>,
    },
    /// Fog on or off for the whole map.
    Fog {
        enabled: bool,
    },
    /// A GM layer becomes visible to all (a secret door found).
    RevealLayer {
        layer: String,
    },
    /// One hidden object or door moves to the first visible layer.
    RevealThing {
        id: String,
    },
    Door {
        door: String,
        state: DoorState,
    },
    /// maps/blend-outdoor-terrain: time, weather, light, live.
    #[serde(rename_all = "camelCase")]
    Ambience {
        #[serde(default)]
        time: Option<TimeOfDay>,
        #[serde(default)]
        weather: Option<Weather>,
        /// `null` clears an override.
        #[serde(default)]
        light: Option<Option<LightLevel>>,
        #[serde(default)]
        sight_limit: Option<Option<u32>>,
    },
    /// The GM moves any token anywhere it can stand.
    MoveToken {
        token: String,
        at: Cell,
    },
    /// Put an NPC of the story on the map.
    PlaceNpc {
        npc: String,
        at: Cell,
        #[serde(default = "yes")]
        hidden: bool,
    },
    RemoveToken {
        token: String,
    },
    #[serde(rename_all = "camelCase")]
    TokenState {
        token: String,
        #[serde(default)]
        hidden: Option<bool>,
        #[serde(default)]
        invisible: Option<bool>,
    },
}

fn yes() -> bool {
    true
}

fn first_visible_layer(map: &Map) -> String {
    map.layers
        .iter()
        .find(|l| l.visibility == Visibility::All)
        .map_or_else(|| "base".to_string(), |l| l.id.clone())
}

/// Apply `edit` to the board.
///
/// # Errors
///
/// 404; 409 `NO_MAP_SHOWN`, `TRAVEL_MAP` (the party token of a world
/// map moves by the journey only); 400 `UNKNOWN_LAYER`, `UNKNOWN_THING`,
/// `UNKNOWN_DOOR`, `UNKNOWN_TOKEN`, `UNKNOWN_NPC`, `CELL_OUTSIDE`,
/// `CANNOT_STAND`.
pub async fn edit(
    pool: &PgPool,
    gm: &CurrentGm,
    campaign: Uuid,
    edit: &Edit,
) -> Result<Board, AppError> {
    let mut tx = pool.begin().await?;
    let row = crate::auth::guard::owned_by(campaigns::lock(&mut tx, campaign).await?, gm)?;
    let mut b = locked(&mut tx, campaign).await?;
    let inside = |b: &Board, c: &Cell| {
        if b.map.grid.contains(*c) {
            Ok(())
        } else {
            Err(AppError::BadRequest("CELL_OUTSIDE"))
        }
    };
    match edit {
        Edit::RevealCells { cells } => {
            for c in cells {
                inside(&b, c)?;
            }
            b.revealed.extend(cells.iter().copied());
        }
        Edit::HideCells { cells } => {
            for c in cells {
                b.revealed.remove(c);
            }
        }
        Edit::Fog { enabled } => b.fog = *enabled,
        Edit::RevealLayer { layer } => {
            let l = b
                .map
                .layers
                .iter_mut()
                .find(|l| &l.id == layer)
                .ok_or(AppError::BadRequest("UNKNOWN_LAYER"))?;
            l.visibility = Visibility::All;
        }
        Edit::RevealThing { id } => {
            let to = first_visible_layer(&b.map);
            if let Some(o) = b.map.objects.iter_mut().find(|o| &o.id == id) {
                o.layer = to;
            } else if let Some(d) = b.map.doors.iter_mut().find(|d| &d.id == id) {
                d.layer = to;
            } else if let Some(p) = b.map.props.iter_mut().find(|p| &p.id == id) {
                p.layer = to;
            } else {
                return Err(AppError::BadRequest("UNKNOWN_THING"));
            }
        }
        Edit::Door { door, state } => {
            if !b.map.set_door_state(door, *state) {
                return Err(AppError::BadRequest("UNKNOWN_DOOR"));
            }
            b.reveal_from_party();
        }
        Edit::Ambience {
            time,
            weather,
            light,
            sight_limit,
        } => {
            let a = &mut b.map.ambience;
            if let Some(t) = time {
                a.time = *t;
            }
            if let Some(w) = weather {
                a.weather = *w;
            }
            if let Some(l) = light {
                a.light = *l;
            }
            if let Some(s) = sight_limit {
                a.sight_limit = *s;
            }
        }
        Edit::MoveToken { token, at } => {
            if token == crate::travel::PARTY_TOKEN && b.map.scale == Scale::World {
                return Err(AppError::Conflict("TRAVEL_MAP"));
            }
            inside(&b, at)?;
            let rules = movement(row.rules(), &b.map);
            promptus_shared::maps::standable(&b.map, &rules, *at)
                .map_err(|_| AppError::BadRequest("CANNOT_STAND"))?;
            let t = b
                .tokens
                .iter_mut()
                .find(|t| &t.id == token)
                .ok_or(AppError::BadRequest("UNKNOWN_TOKEN"))?;
            // The GM lifts the token and sets it down: it glides there.
            if t.at != *at {
                t.walk(&[*at]);
            }
            b.reveal_from_party();
        }
        Edit::PlaceNpc { npc, at, hidden } => {
            inside(&b, at)?;
            let name = row
                .story
                .npc(npc)
                .map(|n| n.name.clone())
                .or_else(|| row.story.adversary(npc).map(|a| a.name.clone()))
                .ok_or(AppError::BadRequest("UNKNOWN_NPC"))?;
            let n = b.tokens.iter().filter(|t| t.r#ref == *npc).count();
            let mut t = Token::new(
                format!("{npc}-{}", n + 1),
                TokenKind::Npc,
                npc.clone(),
                name,
                *at,
                Some(content::npc_look(&row.story, npc)),
            );
            t.hidden = *hidden;
            b.tokens.push(t);
        }
        Edit::RemoveToken { token } => {
            let before = b.tokens.len();
            b.tokens.retain(|t| &t.id != token);
            if b.tokens.len() == before {
                return Err(AppError::BadRequest("UNKNOWN_TOKEN"));
            }
        }
        Edit::TokenState {
            token,
            hidden,
            invisible,
        } => {
            let t = b
                .tokens
                .iter_mut()
                .find(|t| &t.id == token)
                .ok_or(AppError::BadRequest("UNKNOWN_TOKEN"))?;
            if let Some(h) = hidden {
                t.hidden = *h;
            }
            if let Some(i) = invisible {
                t.invisible = *i;
            }
        }
    }
    if b.map.scale == Scale::World
        && matches!(
            edit,
            Edit::RevealCells { .. } | Edit::HideCells { .. } | Edit::MoveToken { .. }
        )
    {
        crate::travel::mirror_fog(&mut tx, campaign, &b.map_id, &b.revealed).await?;
    }
    save(&mut tx, campaign, &b).await?;
    tx.commit().await?;
    Ok(b)
}

/// player/explore-map: `player` walks their character along `path` (the
/// cells entered, in order). Refused during a fight: the fight has its
/// own moves.
///
/// # Errors
///
/// 403 `SPECTATOR`; 404 `NO_CHARACTER`; 409 `NO_MAP_SHOWN`,
/// `NOT_ON_MAP`, `FIGHT_IN_PROGRESS`, `SESSION_NOT_LIVE`, `TRAVEL_MAP`
/// (a world map: the party travels by the journey); 400 `INVALID_PATH`.
pub async fn walk(pool: &PgPool, player: &Player, path: &[Cell]) -> Result<Board, AppError> {
    if player.role != Role::Player {
        return Err(AppError::Forbidden("SPECTATOR"));
    }
    let character = players::character_of(pool, player)
        .await?
        .filter(|c| c.status == CharacterStatus::Validated)
        .ok_or(AppError::NotFound("NO_CHARACTER"))?;
    let mut tx = pool.begin().await?;
    let (row, live) = session::lock_for_player(&mut tx, player, &[Status::Live]).await?;
    if fight::live_id(&mut *tx, player.campaign_id)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("FIGHT_IN_PROGRESS"));
    }
    let mut b = locked(&mut tx, player.campaign_id).await?;
    if b.map.scale == Scale::World {
        return Err(AppError::Conflict("TRAVEL_MAP"));
    }
    let tid = character_token(character.id);
    let from = b
        .tokens
        .iter()
        .find(|t| t.id == tid)
        .map(|t| t.at)
        .ok_or(AppError::Conflict("NOT_ON_MAP"))?;
    let rules = movement(row.rules(), &b.map);
    check_path(
        &b.map,
        &rules,
        &b.occupancy_for(&tid),
        from,
        path,
        EXPLORE_BUDGET,
    )
    .map_err(|_| AppError::BadRequest("INVALID_PATH"))?;
    if let Some(t) = b.tokens.iter_mut().find(|t| t.id == tid) {
        t.walk(path);
    }
    b.reveal_from_party();
    save(&mut tx, player.campaign_id, &b).await?;
    crate::evening::moment(
        &mut tx,
        live.id,
        player.id,
        crate::evening::MomentKind::Move,
    )
    .await?;
    tx.commit().await?;
    Ok(b)
}
