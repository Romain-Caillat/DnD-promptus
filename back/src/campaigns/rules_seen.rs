//! Which version of the campaign's rules each player last read
//! (migration `009_rules_seen.sql`), and what changed since.
//!
//! A seat starts with the version of the moment it was taken (a trigger
//! records it); the player moves it forward by saying they read the
//! changes. Sessions do not exist yet: until they do, the changes show
//! as soon as the campaign's rules move, and the player reads them on
//! their own time — never pushed into a scene.

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::changes::rule_changes;
use promptus_shared::story::RuleSystemRef;
use sqlx::PgPool;
use uuid::Uuid;

use super::projection::rules::ChangesView;
use crate::content;
use crate::error::AppError;

/// The version `player` last read; `None` only for a seat the trigger
/// could not see (none in practice).
///
/// # Errors
///
/// A database error.
pub async fn seen(pool: &PgPool, player: Uuid) -> Result<Option<RuleSystemRef>, AppError> {
    let row: Option<(String, i32)> =
        sqlx::query_as("SELECT rule_system_id, version FROM rules_seen WHERE player_id = $1")
            .bind(player)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(id, version)| RuleSystemRef {
        id,
        version: u32::try_from(version).unwrap_or(0),
    }))
}

/// Records that `player` has read `rules`.
///
/// # Errors
///
/// A database error.
pub async fn mark_seen(pool: &PgPool, player: Uuid, rules: &RuleSystemRef) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO rules_seen (player_id, rule_system_id, version) VALUES ($1, $2, $3)
         ON CONFLICT (player_id) DO UPDATE
         SET rule_system_id = EXCLUDED.rule_system_id, version = EXCLUDED.version, seen_at = now()",
    )
    .bind(player)
    .bind(&rules.id)
    .bind(i32::try_from(rules.version).unwrap_or(i32::MAX))
    .execute(pool)
    .await?;
    Ok(())
}

/// What changed from `seen` to `current` (the campaign's `rules`), or
/// `None` when the player read this very version.
#[must_use]
pub fn changes(
    seen: Option<&RuleSystemRef>,
    current_ref: &RuleSystemRef,
    current: &RuleSystem,
) -> Option<ChangesView> {
    let seen = seen?;
    if seen == current_ref {
        return None;
    }
    let old = (seen.id == current_ref.id)
        .then(|| content::rule_system(seen))
        .flatten();
    Some(ChangesView {
        from_version: seen.version,
        to_version: current_ref.version,
        replaced: old.is_none(),
        items: old.map(|o| rule_changes(o, current)).unwrap_or_default(),
    })
}
