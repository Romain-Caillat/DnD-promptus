//! A campaign's rule system: the preset it started from (`content`), or
//! one of its own versions (`rule_versions`, migration 015).
//!
//! A campaign points at its rules with `story.rules: { id, version }`.
//! The campaign row carries the system that pointer resolves to
//! (`CampaignRow::rules`): the stored text of the campaign's locked
//! version when it has one, the compiled-in preset otherwise. A locked
//! version never changes, so its parsed form is kept in memory by
//! `(campaign, version)`.
//!
//! [`versions`] is the GM's editor (`campaign/edit-rule-system`):
//! drafts, the report each save produces ([`report`]), locking, and the
//! comparison of two versions ([`diff`]). [`house`] is the co-GM that
//! formalises a house rule (`engine/formalise-house-rules`).

pub mod diff;
pub mod house;
pub mod report;
pub mod versions;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use promptus_shared::rules::RuleSystem;
use promptus_shared::story::RuleSystemRef;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::content;
use crate::error::AppError;

/// Parsed locked versions, by campaign and version: a locked text never
/// changes, so it is parsed once per process.
type Parsed = HashMap<(Uuid, u32), Arc<RuleSystem>>;

static PARSED: LazyLock<Mutex<Parsed>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The rule system `r` of `campaign`: `stored` is the text of the
/// campaign's locked version `r.version`, when it has one; otherwise
/// the preset `r` names. `None` when neither exists, or when a stored
/// text no longer loads (logged: the engine changed under it).
pub(crate) fn resolve(
    campaign: Uuid,
    r: &RuleSystemRef,
    stored: Option<&str>,
) -> Option<Arc<RuleSystem>> {
    let Some(text) = stored else {
        return content::preset(r).cloned();
    };
    let key = (campaign, r.version);
    let mut parsed = PARSED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(system) = parsed.get(&key) {
        return Some(system.clone());
    }
    match RuleSystem::from_yaml(text) {
        Ok(system) if system.id == r.id && system.version == r.version => {
            let system = Arc::new(system);
            parsed.insert(key, system.clone());
            Some(system)
        }
        Ok(system) => {
            tracing::error!(%campaign, stored = %format!("{}@{}", system.id, system.version), wanted = %format!("{}@{}", r.id, r.version), "stored rule version names another system");
            None
        }
        Err(e) => {
            tracing::error!(%campaign, version = r.version, error = %e, "stored rule version no longer loads");
            None
        }
    }
}

/// Version `r` of `campaign`'s rules if it is locked or a preset: what
/// a player read before (`campaigns::rules_seen`).
///
/// # Errors
///
/// A database error.
pub async fn locked(
    db: impl PgExecutor<'_>,
    campaign: Uuid,
    r: &RuleSystemRef,
) -> Result<Option<Arc<RuleSystem>>, AppError> {
    let text: Option<String> = sqlx::query_scalar(
        "SELECT system FROM rule_versions
         WHERE campaign_id = $1 AND version = $2 AND locked_at IS NOT NULL",
    )
    .bind(campaign)
    .bind(i32::try_from(r.version).unwrap_or(i32::MAX))
    .fetch_optional(db)
    .await?;
    Ok(resolve(campaign, r, text.as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(id: &str, version: u32) -> RuleSystemRef {
        RuleSystemRef {
            id: id.into(),
            version,
        }
    }

    #[test]
    fn without_a_stored_version_the_preset_answers() {
        let c = Uuid::new_v4();
        for id in ["corsaires", "brasier", "srd"] {
            assert_eq!(resolve(c, &r(id, 1), None).unwrap().id, id);
        }
        assert!(resolve(c, &r("corsaires", 99), None).is_none());
    }

    #[test]
    fn a_stored_version_is_read_and_must_be_the_one_named() {
        let c = Uuid::new_v4();
        let v2 = content::preset_yaml(&r("corsaires", 1)).unwrap().replacen(
            "version: 1",
            "version: 2",
            1,
        );
        let system = resolve(c, &r("corsaires", 2), Some(&v2)).unwrap();
        assert_eq!((system.id.as_str(), system.version), ("corsaires", 2));
        // A text that says another version is not this one.
        let other = Uuid::new_v4();
        assert!(resolve(other, &r("corsaires", 3), Some(&v2)).is_none());
        assert!(resolve(other, &r("corsaires", 2), Some("pas: [des règles")).is_none());
    }
}
