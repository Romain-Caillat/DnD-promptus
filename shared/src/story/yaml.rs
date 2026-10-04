//! YAML import and export of a whole campaign — the format the GM and
//! the next tickets write by hand (`docs/campaign-format.md`).

use super::model::Campaign;

/// A YAML text that is not a campaign: a syntax error, a missing field,
/// an unknown key, a value of the wrong type. The message carries the
/// line and column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlError(pub String);

impl std::fmt::Display for YamlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for YamlError {}

/// Parse a campaign. Only the shape is checked here; what the content
/// means is `validate::validate`'s job, which never refuses.
///
/// # Errors
///
/// When the text is not valid YAML or does not have the campaign shape.
pub fn from_yaml(text: &str) -> Result<Campaign, YamlError> {
    serde_yaml_ng::from_str(text).map_err(|e| YamlError(e.to_string()))
}

/// Write a campaign as YAML. Empty optional fields are left out, so
/// `from_yaml(&to_yaml(c)) == c`.
///
/// # Errors
///
/// Never in practice: every field of the model is representable.
pub fn to_yaml(campaign: &Campaign) -> Result<String, YamlError> {
    serde_yaml_ng::to_string(campaign).map_err(|e| YamlError(e.to_string()))
}
