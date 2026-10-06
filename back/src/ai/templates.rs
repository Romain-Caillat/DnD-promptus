//! Versioned prompt templates (`back/prompts/<id>.v<version>.md`).
//!
//! A prompt is code the model runs: changing one changes what the GM
//! gets. Each template carries an id and a version, recorded with every
//! call (`ai_calls.template`); a change of wording is a new version
//! file, never an edit of an old one, so a history row always points to
//! the text that produced it.
//!
//! A file holds the system message, a line `---user---`, and the user
//! message. Holes are `{{name}}`; [`Template::render`] refuses a hole
//! left unfilled, so a renamed variable fails loudly in tests.

use std::collections::BTreeMap;

use super::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Template {
    pub id: &'static str,
    pub version: u32,
    text: &'static str,
}

/// The co-GM in a live session (`copilot/draft-narration`).
pub const COPILOT: Template = Template {
    id: "copilot",
    version: 1,
    text: include_str!("../../prompts/copilot.v1.md"),
};

/// End-of-session recap drafts (`session/end-session`).
pub const RECAP: Template = Template {
    id: "recap",
    version: 1,
    text: include_str!("../../prompts/recap.v1.md"),
};

/// A pixel-art image (`media/draw-pixel-art-assets`).
pub const PIXEL_ART: Template = Template {
    id: "pixel-art",
    version: 1,
    text: include_str!("../../prompts/pixel-art.v1.md"),
};

/// The co-GM's workshop in prep (`campaign/review-story-graph`).
pub const WORKSHOP: Template = Template {
    id: "workshop",
    version: 1,
    text: include_str!("../../prompts/workshop.v1.md"),
};

/// Every template, for the tests that check each one renders.
pub const ALL: [Template; 4] = [COPILOT, RECAP, PIXEL_ART, WORKSHOP];

const USER_MARK: &str = "---user---";

impl Template {
    /// `copilot@1`, as stored with each call.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}@{}", self.id, self.version)
    }

    fn parts(&self) -> (&'static str, &'static str) {
        self.text
            .split_once(USER_MARK)
            .map_or(("", self.text), |(s, u)| (s.trim(), u.trim()))
    }

    /// The holes the template expects, sorted.
    #[must_use]
    pub fn holes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut rest = self.text;
        while let Some(start) = rest.find("{{") {
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else { break };
            let name = after[..end].trim().to_string();
            if !out.contains(&name) {
                out.push(name);
            }
            rest = &after[end + 2..];
        }
        out.sort();
        out
    }

    /// The messages, every hole filled from `vars`.
    ///
    /// # Errors
    ///
    /// The name of the first hole `vars` does not fill.
    pub fn render(&self, vars: &BTreeMap<&str, String>) -> Result<Vec<Message>, String> {
        let fill = |text: &str| -> Result<String, String> {
            let mut out = String::with_capacity(text.len());
            let mut rest = text;
            while let Some(start) = rest.find("{{") {
                out.push_str(&rest[..start]);
                let after = &rest[start + 2..];
                let end = after
                    .find("}}")
                    .ok_or_else(|| format!("unclosed hole in {}", self.key()))?;
                let name = after[..end].trim();
                let value = vars
                    .get(name)
                    .ok_or_else(|| format!("{}: no value for {{{{{name}}}}}", self.key()))?;
                out.push_str(value);
                rest = &after[end + 2..];
            }
            out.push_str(rest);
            Ok(out)
        };
        let (system, user) = self.parts();
        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(Message::system(fill(system)?));
        }
        messages.push(Message::user(fill(user)?));
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_renders_once_its_holes_are_filled_and_not_before() {
        for template in ALL {
            let holes = template.holes();
            assert!(!holes.is_empty(), "{}", template.key());
            let mut vars: BTreeMap<&str, String> = holes
                .iter()
                .map(|h| (h.as_str(), format!("<{h}>")))
                .collect();
            let messages = template.render(&vars).unwrap();
            let all: String = messages.iter().map(|m| m.content.clone()).collect();
            assert!(!all.contains("{{"), "{}", template.key());
            for h in &holes {
                assert!(all.contains(&format!("<{h}>")), "{} {h}", template.key());
            }
            let first = holes[0].clone();
            vars.remove(first.as_str());
            assert!(template.render(&vars).is_err(), "{}", template.key());
        }
    }
}
