//! The rule systems the server knows, compiled into the binary
//! (`content/rules/<id>/v<n>.yaml`). A campaign points at one with its
//! `rules: { id, version }`; [`system`] resolves that pointer.
//!
//! Editing a system (`campaign/edit-rule-system`) will store GM versions
//! in the database; until then the two witness worlds' drafts are the
//! catalogue.

use std::sync::LazyLock;

use promptus_shared::rules::RuleSystem;
use promptus_shared::story::RuleSystemRef;

const SYSTEMS: [&str; 2] = [
    include_str!("../../content/rules/corsaires/v1.yaml"),
    include_str!("../../content/rules/brasier/v1.yaml"),
];

static CATALOGUE: LazyLock<Vec<RuleSystem>> = LazyLock::new(|| {
    // `shared/tests/rules_worlds.rs` loads the same files: a broken
    // system never reaches a build.
    SYSTEMS
        .iter()
        .map(|t| RuleSystem::from_yaml(t).expect("the embedded rule systems load"))
        .collect()
});

/// The rule system `r` points at, if the server knows it.
#[must_use]
pub fn system(r: &RuleSystemRef) -> Option<&'static RuleSystem> {
    CATALOGUE
        .iter()
        .find(|s| s.id == r.id.as_str() && s.version == r.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_witness_worlds_resolve_and_nothing_else() {
        for id in ["corsaires", "brasier"] {
            let r = RuleSystemRef {
                id: id.into(),
                version: 1,
            };
            assert_eq!(system(&r).unwrap().id, id);
        }
        let r = RuleSystemRef {
            id: "corsaires".into(),
            version: 99,
        };
        assert!(system(&r).is_none());
    }
}
