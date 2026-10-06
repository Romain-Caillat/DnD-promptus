//! session/invite-and-join — the people at a campaign's table
//! (migration `004_players.sql`).
//!
//! A GM mints the campaign's invitation link ([`mint_invite`]); whoever
//! opens it picks a nickname and joins ([`join`]) as a player, who gets
//! a draft character, or as a spectator. The device keeps a random token
//! (`auth::player` puts it in a cookie scoped to the campaign); the
//! server keeps its hash only, and finds the player back with it
//! ([`find_by_token`]).
//!
//! Nothing here is sent to a player as is: player routes build their
//! answer in `campaigns::projection`, the single projection point.
//!
//! The GM's review of the sheets (validate, return with a word, the
//! difference since the last review, secret hooks) is in [`review`];
//! a validated character's live play state (hit points, XP, purse, bag)
//! and the history of what changed it are in [`play`].

pub mod assist;
pub mod play;
pub mod review;

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::sprite::{CharacterLook, Direction, Packs, render};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::tokens::{hash_token, random_token};
use crate::error::AppError;
use crate::live::{self, Topic};

/// How long an invitation link opens the door. Players who joined keep
/// their place after it expires.
pub const INVITE_DAYS: i64 = 7;
/// Longest nickname, in characters.
pub const NICKNAME_MAX: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Plays a character.
    Player,
    /// Watches what the shared screen shows; no character.
    Spectator,
}

impl Role {
    fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Spectator => "spectator",
        }
    }

    fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "player" => Ok(Self::Player),
            "spectator" => Ok(Self::Spectator),
            other => Err(AppError::Internal(format!("unknown player role {other}"))),
        }
    }
}

/// Where a character stands between its player and the GM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterStatus {
    /// Being written by the player.
    Draft,
    /// Sent to the GM, waiting for validation.
    Submitted,
    /// Accepted by the GM: it can enter play.
    Validated,
    /// Sent back by the GM with a note (`Character::gm_note`).
    Returned,
}

impl CharacterStatus {
    pub(crate) fn parse(s: &str) -> Result<Self, AppError> {
        match s {
            "draft" => Ok(Self::Draft),
            "submitted" => Ok(Self::Submitted),
            "validated" => Ok(Self::Validated),
            "returned" => Ok(Self::Returned),
            other => Err(AppError::Internal(format!(
                "unknown character status {other}"
            ))),
        }
    }
}

/// What the player wrote about their character. Stored whole in
/// `characters.sheet` and sent back whole to that player, so it holds
/// nothing GM-only. Every field has a default: a fresh draft is `{}`.
///
/// Written by the player in the character creator
/// (`characters/build-character-creator`, [`save_sheet`]) and checked
/// against the rules by `session/validate-characters`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CharacterSheet {
    /// The character's name (« Borin »).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// A people id of the campaign's rule system, when it has peoples.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people_id: Option<String>,
    /// A class id of the campaign's rule system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    /// Ability scores by the rule system's ability id.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub abilities: BTreeMap<String, i32>,
    /// What the character looks like, in the player's words.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub appearance: String,
    /// What the character looks like, as pieces of the campaign's
    /// sprite pack: a description the server draws, never an image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub look: Option<CharacterLook>,
    #[serde(skip_serializing_if = "Backstory::is_empty")]
    pub backstory: Backstory,
}

/// Three short answers instead of a blank page (board « Créer », moment
/// 7), and the paragraph the player makes of them. A sheet written
/// before the questions, with the story as plain text, reads it as the
/// paragraph rather than failing — a sheet that fails to read is
/// checked as empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "BackstoryForm")]
pub struct Backstory {
    /// « D'où vient-il ? »
    #[serde(skip_serializing_if = "String::is_empty")]
    pub origin: String,
    /// « Qui a-t-il perdu ? »
    #[serde(skip_serializing_if = "String::is_empty")]
    pub loss: String,
    /// « Que cherche-t-il ? »
    #[serde(skip_serializing_if = "String::is_empty")]
    pub quest: String,
    /// The story in a paragraph.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
}

/// The two shapes a stored backstory may have.
#[derive(Deserialize)]
#[serde(untagged)]
enum BackstoryForm {
    Text(String),
    Answers {
        #[serde(default)]
        origin: String,
        #[serde(default)]
        loss: String,
        #[serde(default)]
        quest: String,
        #[serde(default)]
        text: String,
    },
}

impl From<BackstoryForm> for Backstory {
    fn from(form: BackstoryForm) -> Self {
        match form {
            BackstoryForm::Text(text) => Self {
                text,
                ..Self::default()
            },
            BackstoryForm::Answers {
                origin,
                loss,
                quest,
                text,
            } => Self {
                origin,
                loss,
                quest,
                text,
            },
        }
    }
}

impl Backstory {
    pub fn is_empty(&self) -> bool {
        self.origin.is_empty()
            && self.loss.is_empty()
            && self.quest.is_empty()
            && self.text.is_empty()
    }
}

/// Longest character name, in characters.
pub const CHARACTER_NAME_MAX: usize = 60;
/// Longest answer to one backstory question, in characters.
pub const ANSWER_MAX: usize = 300;
/// Longest backstory paragraph or appearance text, in characters.
pub const TEXT_MAX: usize = 4000;
/// Ability scores a sheet may hold. The rule system's budget is a
/// warning the GM settles, never a refusal; this only keeps out what no
/// system means.
pub const ABILITY_RANGE: std::ops::RangeInclusive<i32> = 1..=30;

impl CharacterSheet {
    /// The sheet as stored: trimmed name, every id known to `rules`,
    /// a look `packs` can draw from `pack`, texts within their bounds.
    /// What the rules only advise (the ability budget) is not checked
    /// here: it is flagged to the player and settled by the GM.
    ///
    /// # Errors
    ///
    /// 400 with the first problem's code: `INVALID_NAME`,
    /// `UNKNOWN_PEOPLE`, `UNKNOWN_CLASS`, `UNKNOWN_ABILITY`,
    /// `ABILITY_OUT_OF_RANGE`, `TEXT_TOO_LONG`, `SPRITE_WRONG_PACK`, or
    /// the renderer's code for a look it cannot draw.
    pub fn cleaned(
        mut self,
        rules: Option<&RuleSystem>,
        packs: &Packs,
        pack: &str,
    ) -> Result<Self, AppError> {
        let invalid = |code: &'static str, detail: String| AppError::Invalid { code, detail };
        self.name = self.name.trim().to_string();
        if self.name.chars().count() > CHARACTER_NAME_MAX || self.name.chars().any(char::is_control)
        {
            return Err(invalid("INVALID_NAME", self.name));
        }
        if let Some(id) = &self.people_id
            && !rules.is_some_and(|r| r.peoples.iter().any(|p| &p.id == id))
        {
            return Err(invalid("UNKNOWN_PEOPLE", id.clone()));
        }
        if let Some(id) = &self.class_id
            && rules.and_then(|r| r.class(id)).is_none()
        {
            return Err(invalid("UNKNOWN_CLASS", id.clone()));
        }
        for (id, score) in &self.abilities {
            if rules.and_then(|r| r.ability(id)).is_none() {
                return Err(invalid("UNKNOWN_ABILITY", id.clone()));
            }
            if !ABILITY_RANGE.contains(score) {
                return Err(invalid("ABILITY_OUT_OF_RANGE", format!("{id} {score}")));
            }
        }
        let b = &self.backstory;
        let too_long = [&b.origin, &b.loss, &b.quest]
            .into_iter()
            .any(|a| a.chars().count() > ANSWER_MAX)
            || [&b.text, &self.appearance]
                .into_iter()
                .any(|t| t.chars().count() > TEXT_MAX);
        if too_long {
            return Err(AppError::BadRequest("TEXT_TOO_LONG"));
        }
        if let Some(look) = &self.look {
            if look.pack != pack {
                return Err(invalid("SPRITE_WRONG_PACK", look.pack.clone()));
            }
            render(packs, look, Direction::East).map_err(|e| invalid(e.code(), e.to_string()))?;
        }
        Ok(self)
    }

    /// What the sheet still lacks before it can go to the GM: the
    /// fields' names, empty when it is complete.
    pub fn missing(&self, rules: Option<&RuleSystem>) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.name.is_empty() {
            missing.push("name");
        }
        if self.look.is_none() {
            missing.push("look");
        }
        if let Some(rules) = rules {
            if !rules.peoples.is_empty() && self.people_id.is_none() {
                missing.push("peopleId");
            }
            if !rules.classes.is_empty() && self.class_id.is_none() {
                missing.push("classId");
            }
        }
        missing
    }
}

/// Someone who joined a campaign.
#[derive(Debug, Clone)]
pub struct Player {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub nickname: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

/// A player's character slot.
#[derive(Debug, Clone)]
pub struct Character {
    pub id: Uuid,
    pub status: CharacterStatus,
    pub sheet: CharacterSheet,
    /// The GM's word to the player when the sheet was returned.
    pub gm_note: Option<String>,
    pub updated_at: DateTime<Utc>,
    /// The stored play state, read by [`character_of`] once validated;
    /// `None` also while it was never changed (it is then the starting
    /// state, [`play::PlayState::start`]).
    pub play: Option<play::PlayState>,
}

/// The campaign's active invitation, as the GM sees it. Never the code.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteStatus {
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// A freshly minted invitation: the only time the code is shown.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintedInvite {
    #[serde(flatten)]
    pub status: InviteStatus,
    pub code: String,
}

/// One seat of the table, as the GM's player list shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Seat {
    pub id: Uuid,
    pub nickname: String,
    pub role: Role,
    pub joined_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub character: Option<SeatCharacter>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatCharacter {
    pub id: Uuid,
    pub status: CharacterStatus,
    /// Empty until the player names it.
    pub name: String,
    /// The class id the sheet names, if any.
    pub class_id: Option<String>,
    /// The class's name in the campaign's rule system, filled by the
    /// route (`api::table::seats`).
    pub class_name: Option<String>,
    /// The sheet's `CharacterLook`, as the player stored it, to draw
    /// the seat's sprite.
    pub look: Option<serde_json::Value>,
    /// Sent again after the GM returned it: the review shows only the
    /// difference.
    pub resubmitted: bool,
    pub updated_at: DateTime<Utc>,
}

// --- Invitation ------------------------------------------------------------

/// Mint a new invitation for `campaign_id`, replacing (and so revoking)
/// the previous one. The caller has checked the GM owns the campaign.
///
/// # Errors
///
/// Fails on a database error.
pub async fn mint_invite(pool: &PgPool, campaign_id: Uuid) -> Result<MintedInvite, AppError> {
    let code = random_token();
    let expires_at = Utc::now() + Duration::days(INVITE_DAYS);
    let (created_at, expires_at): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO campaign_invites (campaign_id, token_hash, expires_at) VALUES ($1, $2, $3)
         ON CONFLICT (campaign_id) DO UPDATE
           SET token_hash = EXCLUDED.token_hash, created_at = now(), expires_at = EXCLUDED.expires_at
         RETURNING created_at, expires_at",
    )
    .bind(campaign_id)
    .bind(hash_token(&code))
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(MintedInvite {
        status: InviteStatus {
            created_at,
            expires_at,
        },
        code,
    })
}

/// The campaign's usable invitation, if any.
///
/// # Errors
///
/// Fails on a database error.
pub async fn invite_status(
    pool: &PgPool,
    campaign_id: Uuid,
) -> Result<Option<InviteStatus>, AppError> {
    let row: Option<(DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT created_at, expires_at FROM campaign_invites
         WHERE campaign_id = $1 AND expires_at > now()",
    )
    .bind(campaign_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(created_at, expires_at)| InviteStatus {
        created_at,
        expires_at,
    }))
}

/// Close the door: the link stops working. Idempotent.
///
/// # Errors
///
/// Fails on a database error.
pub async fn revoke_invite(pool: &PgPool, campaign_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM campaign_invites WHERE campaign_id = $1")
        .bind(campaign_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// The campaign a usable invitation code opens, if any.
///
/// # Errors
///
/// Fails on a database error.
pub async fn campaign_by_invite(pool: &PgPool, code: &str) -> Result<Option<Uuid>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT campaign_id FROM campaign_invites
         WHERE token_hash = $1 AND expires_at > now()",
    )
    .bind(hash_token(code))
    .fetch_optional(pool)
    .await?)
}

// --- Joining -----------------------------------------------------------------

/// A nickname as stored: trimmed, 1 to [`NICKNAME_MAX`] characters, no
/// control characters.
///
/// # Errors
///
/// 400 `INVALID_NICKNAME` otherwise.
pub fn clean_nickname(raw: &str) -> Result<String, AppError> {
    let nick = raw.trim();
    let len = nick.chars().count();
    if len == 0 || len > NICKNAME_MAX || nick.chars().any(char::is_control) {
        return Err(AppError::BadRequest("INVALID_NICKNAME"));
    }
    Ok(nick.to_string())
}

type PlayerRow = (Uuid, Uuid, String, String, DateTime<Utc>, DateTime<Utc>);

fn player_from_row(
    (id, campaign_id, nickname, role, created_at, last_seen_at): PlayerRow,
) -> Result<Player, AppError> {
    Ok(Player {
        id,
        campaign_id,
        nickname,
        role: Role::parse(&role)?,
        created_at,
        last_seen_at,
    })
}

const PLAYER_COLUMNS: &str = "id, campaign_id, nickname, role, created_at, last_seen_at";

/// Seat `nickname` at `campaign_id`'s table as `role`; a player gets an
/// empty draft character in the same transaction. Returns the player
/// and the raw device token, which only the device will keep.
///
/// # Errors
///
/// 400 `INVALID_NICKNAME`; 400 `NICKNAME_TAKEN` when someone at this
/// table already goes by that name (case aside); a database error.
pub async fn join(
    pool: &PgPool,
    campaign_id: Uuid,
    nickname: &str,
    role: Role,
) -> Result<(Player, String), AppError> {
    let nickname = clean_nickname(nickname)?;
    let token = random_token();
    let mut tx = pool.begin().await?;
    let row: Result<PlayerRow, sqlx::Error> = sqlx::query_as(&format!(
        "INSERT INTO players (campaign_id, nickname, role, token_hash) VALUES ($1, $2, $3, $4)
         RETURNING {PLAYER_COLUMNS}"
    ))
    .bind(campaign_id)
    .bind(&nickname)
    .bind(role.as_str())
    .bind(hash_token(&token))
    .fetch_one(&mut *tx)
    .await;
    let player = match row {
        Ok(row) => player_from_row(row)?,
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("players_nickname_idx") => {
            return Err(AppError::BadRequest("NICKNAME_TAKEN"));
        }
        Err(e) => return Err(e.into()),
    };
    if role == Role::Player {
        create_character(&mut tx, &player).await?;
    } else {
        live::touch(&mut tx, campaign_id, &Topic::Table).await?;
    }
    tx.commit().await?;
    Ok((player, token))
}

async fn create_character(
    tx: &mut Transaction<'_, Postgres>,
    player: &Player,
) -> Result<(), AppError> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO characters (campaign_id, player_id, sheet) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(player.campaign_id)
    .bind(player.id)
    .bind(Json(CharacterSheet::default()))
    .fetch_one(&mut **tx)
    .await?;
    touch_character(tx, player.campaign_id, id).await?;
    Ok(())
}

/// Tell whoever displays character `id` of `campaign` that it changed:
/// its own topic, and the table (`live::Topic::Table`), whose seats show
/// each character's name and status. Call it from every write to a
/// character, inside its transaction — the player's draft saves and
/// submissions as well as the GM's review.
///
/// # Errors
///
/// Fails on a database error.
pub async fn touch_character(
    tx: &mut Transaction<'_, Postgres>,
    campaign: Uuid,
    id: Uuid,
) -> Result<(), AppError> {
    live::touch(tx, campaign, &Topic::Character(id)).await?;
    live::touch(tx, campaign, &Topic::Table).await?;
    Ok(())
}

/// The player of `campaign_id` holding one of `tokens` (a browser may
/// send several cookies), marked as seen now.
///
/// # Errors
///
/// Fails on a database error.
pub async fn find_by_token(
    pool: &PgPool,
    campaign_id: Uuid,
    tokens: &[String],
) -> Result<Option<Player>, AppError> {
    if tokens.is_empty() {
        return Ok(None);
    }
    let hashes: Vec<String> = tokens.iter().map(|t| hash_token(t)).collect();
    let row: Option<PlayerRow> = sqlx::query_as(&format!(
        "UPDATE players SET last_seen_at = now()
         WHERE campaign_id = $1 AND token_hash = ANY($2)
         RETURNING {PLAYER_COLUMNS}"
    ))
    .bind(campaign_id)
    .bind(&hashes)
    .fetch_optional(pool)
    .await?;
    row.map(player_from_row).transpose()
}

/// `player`'s character, if they have one.
///
/// # Errors
///
/// Fails on a database error.
pub async fn character_of(pool: &PgPool, player: &Player) -> Result<Option<Character>, AppError> {
    let row: Option<CharacterRow> = sqlx::query_as(&format!(
        "SELECT {CHARACTER_COLUMNS} FROM characters WHERE player_id = $1"
    ))
    .bind(player.id)
    .fetch_optional(pool)
    .await?;
    let Some(mut character) = row.map(character_from_row).transpose()? else {
        return Ok(None);
    };
    if character.status == CharacterStatus::Validated {
        character.play = play::stored(pool, character.id).await?;
    }
    Ok(Some(character))
}

type CharacterRow = (
    Uuid,
    String,
    Json<CharacterSheet>,
    Option<String>,
    DateTime<Utc>,
);

const CHARACTER_COLUMNS: &str = "id, status, sheet, gm_note, updated_at";

fn character_from_row(
    (id, status, sheet, gm_note, updated_at): CharacterRow,
) -> Result<Character, AppError> {
    Ok(Character {
        id,
        status: CharacterStatus::parse(&status)?,
        sheet: sheet.0,
        gm_note,
        updated_at,
        play: None,
    })
}

impl CharacterStatus {
    /// Whether the player may still change the sheet: while writing it,
    /// and when the GM sent it back. Once sent, it is the GM's turn.
    pub fn editable(self) -> bool {
        matches!(self, Self::Draft | Self::Returned)
    }
}

/// `player`'s character row, locked until the transaction ends.
async fn lock_character(
    tx: &mut Transaction<'_, Postgres>,
    player: &Player,
) -> Result<Character, AppError> {
    let row: Option<CharacterRow> = sqlx::query_as(&format!(
        "SELECT {CHARACTER_COLUMNS} FROM characters WHERE player_id = $1 FOR UPDATE"
    ))
    .bind(player.id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(character_from_row)
        .transpose()?
        .ok_or(AppError::NotFound("NO_CHARACTER"))
}

// --- The player's writes -----------------------------------------------------

/// Save the player's draft: the whole sheet, already
/// [`CharacterSheet::cleaned`]. Only while the sheet is theirs to edit
/// (draft, or returned by the GM); the status does not change.
///
/// # Errors
///
/// 404 `NO_CHARACTER` (a spectator); 409 `CHARACTER_LOCKED` once sent
/// to the GM or validated; a database error.
pub async fn save_sheet(
    pool: &PgPool,
    player: &Player,
    sheet: &CharacterSheet,
) -> Result<Character, AppError> {
    let mut tx = pool.begin().await?;
    let current = lock_character(&mut tx, player).await?;
    if !current.status.editable() {
        return Err(AppError::Conflict("CHARACTER_LOCKED"));
    }
    let row: CharacterRow = sqlx::query_as(&format!(
        "UPDATE characters SET sheet = $2 WHERE id = $1 RETURNING {CHARACTER_COLUMNS}"
    ))
    .bind(current.id)
    .bind(Json(sheet))
    .fetch_one(&mut *tx)
    .await?;
    touch_character(&mut tx, player.campaign_id, current.id).await?;
    tx.commit().await?;
    character_from_row(row)
}

/// Send the character to the GM: draft or returned → submitted. The one
/// place a sheet goes to the GM, so `session/validate-characters` hooks
/// what it keeps of each submission here. A sheet over the rules'
/// budget goes all the same: the GM decides.
///
/// # Errors
///
/// 404 `NO_CHARACTER`; 409 `CHARACTER_LOCKED` when already sent or
/// validated; 400 `CHARACTER_INCOMPLETE` (the missing fields in the
/// detail) without a name, a look, or the people and class the rules
/// ask for; a database error.
pub async fn submit_character(
    pool: &PgPool,
    player: &Player,
    rules: Option<&RuleSystem>,
) -> Result<Character, AppError> {
    let mut tx = pool.begin().await?;
    let current = lock_character(&mut tx, player).await?;
    if !current.status.editable() {
        return Err(AppError::Conflict("CHARACTER_LOCKED"));
    }
    let missing = current.sheet.missing(rules);
    if !missing.is_empty() {
        return Err(AppError::Invalid {
            code: "CHARACTER_INCOMPLETE",
            detail: missing.join(","),
        });
    }
    let row: CharacterRow = sqlx::query_as(&format!(
        "UPDATE characters SET status = 'submitted' WHERE id = $1 RETURNING {CHARACTER_COLUMNS}"
    ))
    .bind(current.id)
    .fetch_one(&mut *tx)
    .await?;
    touch_character(&mut tx, player.campaign_id, current.id).await?;
    tx.commit().await?;
    character_from_row(row)
}

// --- The GM's view of the table ----------------------------------------------

/// Everyone at `campaign_id`'s table, in order of arrival. The caller
/// has checked the GM owns the campaign.
///
/// # Errors
///
/// Fails on a database error.
pub async fn seats(pool: &PgPool, campaign_id: Uuid) -> Result<Vec<Seat>, AppError> {
    type SeatRow = (
        Uuid,
        String,
        String,
        DateTime<Utc>,
        DateTime<Utc>,
        Option<Uuid>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<serde_json::Value>,
        Option<bool>,
        Option<DateTime<Utc>>,
    );
    let rows: Vec<SeatRow> = sqlx::query_as(
        "SELECT p.id, p.nickname, p.role, p.created_at, p.last_seen_at,
                c.id, c.status, c.sheet->>'name', c.sheet->>'classId', c.sheet->'look',
                c.reviewed_sheet IS NOT NULL, c.updated_at
         FROM players p LEFT JOIN characters c ON c.player_id = p.id
         WHERE p.campaign_id = $1
         ORDER BY p.created_at, p.id",
    )
    .bind(campaign_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(
            |(
                id,
                nickname,
                role,
                joined_at,
                last_seen_at,
                cid,
                status,
                name,
                class_id,
                look,
                reviewed,
                updated_at,
            )| {
                let character = match (cid, status, updated_at) {
                    (Some(id), Some(status), Some(updated_at)) => {
                        let status = CharacterStatus::parse(&status)?;
                        Some(SeatCharacter {
                            id,
                            status,
                            name: name.unwrap_or_default(),
                            class_id,
                            class_name: None,
                            look,
                            resubmitted: status == CharacterStatus::Submitted
                                && reviewed.unwrap_or(false),
                            updated_at,
                        })
                    }
                    _ => None,
                };
                Ok(Seat {
                    id,
                    nickname,
                    role: Role::parse(&role)?,
                    joined_at,
                    last_seen_at,
                    character,
                })
            },
        )
        .collect()
}

/// Remove a player from `campaign_id`'s table, with their character:
/// their device token stops working. The caller has checked the GM owns
/// the campaign.
///
/// # Errors
///
/// 404 `NOT_FOUND` when no such player sits at this table.
pub async fn remove(pool: &PgPool, campaign_id: Uuid, player_id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let character: Option<Option<Uuid>> = sqlx::query_scalar(
        "DELETE FROM players p WHERE p.id = $1 AND p.campaign_id = $2
         RETURNING (SELECT c.id FROM characters c WHERE c.player_id = p.id)",
    )
    .bind(player_id)
    .bind(campaign_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(character) = character else {
        return Err(AppError::NotFound("NOT_FOUND"));
    };
    // Whoever displays that character learns it is gone.
    if let Some(id) = character {
        touch_character(&mut tx, campaign_id, id).await?;
    } else {
        live::touch(&mut tx, campaign_id, &Topic::Table).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nicknames_are_trimmed_and_bounded() {
        assert_eq!(clean_nickname("  Marc ").unwrap(), "Marc");
        assert!(clean_nickname("   ").is_err());
        assert!(clean_nickname("a\u{0}b").is_err());
        assert!(clean_nickname(&"é".repeat(NICKNAME_MAX)).is_ok());
        assert!(clean_nickname(&"é".repeat(NICKNAME_MAX + 1)).is_err());
    }

    #[test]
    fn an_empty_sheet_reads_back_from_an_empty_object() {
        let sheet: CharacterSheet = serde_json::from_str("{}").unwrap();
        assert_eq!(sheet, CharacterSheet::default());
        assert_eq!(serde_json::to_string(&sheet).unwrap(), "{}");
    }

    #[test]
    fn a_backstory_reads_from_answers_or_from_plain_text() {
        let answers: CharacterSheet = serde_json::from_str(
            r#"{"name":"Borin","backstory":{"origin":"La mine.","text":"Nain."}}"#,
        )
        .unwrap();
        assert_eq!(answers.backstory.origin, "La mine.");
        assert_eq!(answers.backstory.text, "Nain.");
        // Written before the questions: the rest of the sheet survives.
        let text: CharacterSheet =
            serde_json::from_str(r#"{"name":"Borin","backstory":"La mine."}"#).unwrap();
        assert_eq!(text.name, "Borin");
        assert_eq!(text.backstory.text, "La mine.");
        assert_eq!(
            serde_json::to_value(&text).unwrap()["backstory"],
            serde_json::json!({ "text": "La mine." })
        );
    }
}
