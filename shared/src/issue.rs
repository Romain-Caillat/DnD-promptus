//! What a check reports. The campaign validator (`story::validate`) and
//! the rule-system lint (`rules::lint`) share this shape so the GM's
//! screens show both the same way. Checks **report, never block**
//! (`docs/lecons-des-parties.md` §3): the GM decides.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something is broken: a reference to nothing, a duplicate id.
    Error,
    /// A design flaw the GM should look at.
    Warning,
    /// Worth a look, possibly intended: the GM confirms.
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub severity: Severity,
    /// Stable machine code, `UPPER_SNAKE` (`REF_DANGLING`, `DAMAGE_MODEL_MIXED`).
    pub code: &'static str,
    /// Where in the document, as `adversaries[garde_royal].armor_class`.
    pub path: String,
    /// English, for developers and logs.
    pub detail: String,
    /// French, for the GM, when the check writes one (the rule-system
    /// lint does; the campaign validator leaves it to the UI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
