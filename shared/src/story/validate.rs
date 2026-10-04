//! Checks on a campaign. They **report, never block**: a campaign with
//! issues is stored and editable all the same; the GM decides
//! (`docs/lecons-des-parties.md` §3, checks flag but never block).
//!
//! Each issue has a stable machine `code` (the French UI translates it
//! later), a `path` into the document and an English `detail`.
//!
//! What is checked:
//! - ids: format, and uniqueness across the whole campaign (one
//!   namespace);
//! - references: every id a field points at exists **and** is of the
//!   kind that field expects;
//! - the three-clue rule: a critical revelation is reachable through
//!   clues placed in at least three different nodes;
//! - required knowledge: what a node says the players must know
//!   entering is given somewhere else — and not only in optional
//!   scenes (the Greyhound route of the Corsaires' act 1);
//! - structure: front clocks of 4–6 steps, an opening node, nodes
//!   reachable from it, exits that go somewhere else, loot that is
//!   something, sane affinity bounds, music that is a YouTube link (or
//!   a track still to choose);
//! - players: every party member has a hook in each act with scenes.
//!
//! Rule-system data (stats, difficulties, damage formulas) is checked
//! by the rules engine against the campaign's rule system, not here.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use serde::Serialize;

use super::model::{Campaign, FORMAT_VERSION, Importance, MusicTrack};

pub const MIN_CRITICAL_CLUE_NODES: usize = 3;
pub const FRONT_STEPS_MIN: usize = 4;
pub const FRONT_STEPS_MAX: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something is broken: a reference to nothing, a duplicate id.
    Error,
    /// A design flaw the GM should look at.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    pub path: String,
    pub detail: String,
}

/// The kinds of things an id can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Campaign,
    PartyMember,
    Act,
    Front,
    Node,
    Revelation,
    Clue,
    Npc,
    Adversary,
    Location,
    Item,
    Faction,
    Goal,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Campaign => "campaign",
            Self::PartyMember => "party member",
            Self::Act => "act",
            Self::Front => "front",
            Self::Node => "node",
            Self::Revelation => "revelation",
            Self::Clue => "clue",
            Self::Npc => "npc",
            Self::Adversary => "adversary",
            Self::Location => "location",
            Self::Item => "item",
            Self::Faction => "faction",
            Self::Goal => "goal",
        }
    }
}

struct Checker<'a> {
    kinds: HashMap<&'a str, Kind>,
    issues: Vec<Issue>,
}

impl<'a> Checker<'a> {
    fn push(&mut self, severity: Severity, code: &'static str, path: String, detail: String) {
        self.issues.push(Issue {
            severity,
            code,
            path,
            detail,
        });
    }

    fn error(&mut self, code: &'static str, path: String, detail: String) {
        self.push(Severity::Error, code, path, detail);
    }

    fn warn(&mut self, code: &'static str, path: String, detail: String) {
        self.push(Severity::Warning, code, path, detail);
    }

    fn declare(&mut self, id: &'a str, kind: Kind, path: String) {
        if !valid_id(id) {
            self.error(
                "ID_INVALID",
                format!("{path}.id"),
                format!("`{id}`: ids are lowercase letters, digits, `_` and `-`, starting with a letter or digit"),
            );
        }
        if let Some(previous) = self.kinds.insert(id, kind) {
            self.kinds.insert(id, previous);
            self.error(
                "ID_DUPLICATE",
                format!("{path}.id"),
                format!(
                    "`{id}` is already the id of a {}; ids are unique across the whole campaign",
                    previous.name()
                ),
            );
        }
    }

    /// `id`, found at `path`, must name one of `expected`.
    fn reference(&mut self, id: &str, path: String, expected: &[Kind]) {
        match self.kinds.get(id) {
            None => self.error(
                "REF_DANGLING",
                path,
                format!("`{id}` names nothing in this campaign"),
            ),
            Some(kind) if !expected.contains(kind) => {
                let wanted: Vec<_> = expected.iter().map(|k| k.name()).collect();
                let detail = format!(
                    "`{id}` is a {}, expected a {}",
                    kind.name(),
                    wanted.join(" or ")
                );
                self.error("REF_WRONG_KIND", path, detail);
            }
            Some(_) => {}
        }
    }
}

fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn is_youtube(url: &str) -> bool {
    [
        "https://www.youtube.com/",
        "https://youtube.com/",
        "https://youtu.be/",
        "https://music.youtube.com/",
    ]
    .iter()
    .any(|p| url.starts_with(p))
}

/// Every issue of `campaign`, in document order within each check.
#[must_use]
pub fn validate(campaign: &Campaign) -> Vec<Issue> {
    let mut ck = Checker {
        kinds: HashMap::new(),
        issues: Vec::new(),
    };
    let c = campaign;

    if c.format != FORMAT_VERSION {
        ck.error(
            "FORMAT_UNSUPPORTED",
            "format".into(),
            format!(
                "format {} is not {FORMAT_VERSION}, the version this server reads",
                c.format
            ),
        );
    }

    // --- Ids ---------------------------------------------------------------
    ck.declare(&c.id, Kind::Campaign, "campaign".into());
    for (i, x) in c.party.iter().enumerate() {
        ck.declare(&x.id, Kind::PartyMember, format!("party[{i}]"));
    }
    for (i, x) in c.acts.iter().enumerate() {
        ck.declare(&x.id, Kind::Act, format!("acts[{i}]"));
    }
    for (i, x) in c.fronts.iter().enumerate() {
        ck.declare(&x.id, Kind::Front, format!("fronts[{i}]"));
    }
    for (i, x) in c.nodes.iter().enumerate() {
        ck.declare(&x.id, Kind::Node, format!("nodes[{i}]"));
    }
    for (i, x) in c.revelations.iter().enumerate() {
        ck.declare(&x.id, Kind::Revelation, format!("revelations[{i}]"));
    }
    for (i, x) in c.clues.iter().enumerate() {
        ck.declare(&x.id, Kind::Clue, format!("clues[{i}]"));
    }
    for (i, x) in c.npcs.iter().enumerate() {
        ck.declare(&x.id, Kind::Npc, format!("npcs[{i}]"));
    }
    for (i, x) in c.adversaries.iter().enumerate() {
        ck.declare(&x.id, Kind::Adversary, format!("adversaries[{i}]"));
    }
    for (i, x) in c.locations.iter().enumerate() {
        ck.declare(&x.id, Kind::Location, format!("locations[{i}]"));
    }
    for (i, x) in c.items.iter().enumerate() {
        ck.declare(&x.id, Kind::Item, format!("items[{i}]"));
    }
    for (i, x) in c.factions.iter().enumerate() {
        ck.declare(&x.id, Kind::Faction, format!("factions[{i}]"));
    }
    for (i, x) in c.goals.iter().enumerate() {
        ck.declare(&x.id, Kind::Goal, format!("goals[{i}]"));
    }

    check_references(&mut ck, c);
    check_structure(&mut ck, c);
    check_clues(&mut ck, c);
    check_required_knowledge(&mut ck, c);
    check_player_hooks(&mut ck, c);
    ck.issues
}

fn check_references(ck: &mut Checker<'_>, c: &Campaign) {
    if let Some(start) = &c.bible.start_node {
        ck.reference(start, "bible.start_node".into(), &[Kind::Node]);
    }
    for (i, n) in c.nodes.iter().enumerate() {
        let p = format!("nodes[{i}]");
        ck.reference(&n.act, format!("{p}.act"), &[Kind::Act]);
        if let Some(l) = &n.location {
            ck.reference(l, format!("{p}.location"), &[Kind::Location]);
        }
        for (j, x) in n.npcs.iter().enumerate() {
            ck.reference(&x.npc, format!("{p}.npcs[{j}].npc"), &[Kind::Npc]);
        }
        if let Some(e) = &n.encounter {
            for (j, o) in e.opponents.iter().enumerate() {
                ck.reference(
                    &o.who,
                    format!("{p}.encounter.opponents[{j}].who"),
                    &[Kind::Adversary, Kind::Npc],
                );
            }
        }
        for (j, l) in n.loot.iter().enumerate() {
            if let Some(item) = &l.item {
                ck.reference(item, format!("{p}.loot[{j}].item"), &[Kind::Item]);
            }
        }
        for (j, x) in n.exits.iter().enumerate() {
            ck.reference(&x.to, format!("{p}.exits[{j}].to"), &[Kind::Node]);
        }
        for (j, r) in n.requires.iter().enumerate() {
            ck.reference(r, format!("{p}.requires[{j}]"), &[Kind::Revelation]);
        }
        for (j, h) in n.player_hooks.iter().enumerate() {
            ck.reference(
                &h.character,
                format!("{p}.player_hooks[{j}].character"),
                &[Kind::PartyMember],
            );
        }
    }
    for (i, x) in c.clues.iter().enumerate() {
        let p = format!("clues[{i}]");
        ck.reference(
            &x.revelation,
            format!("{p}.revelation"),
            &[Kind::Revelation],
        );
        ck.reference(&x.node, format!("{p}.node"), &[Kind::Node]);
        if let Some(s) = &x.source {
            ck.reference(
                s,
                format!("{p}.source"),
                &[Kind::Npc, Kind::Item, Kind::Location],
            );
        }
    }
    for (i, n) in c.npcs.iter().enumerate() {
        let p = format!("npcs[{i}]");
        for (j, h) in n.inventory.iter().enumerate() {
            ck.reference(&h.item, format!("{p}.inventory[{j}].item"), &[Kind::Item]);
        }
        for (j, s) in n.sells.iter().enumerate() {
            ck.reference(&s.item, format!("{p}.sells[{j}].item"), &[Kind::Item]);
        }
        if let Some(f) = &n.faction {
            ck.reference(f, format!("{p}.faction"), &[Kind::Faction]);
        }
        if let Some(l) = &n.location {
            ck.reference(l, format!("{p}.location"), &[Kind::Location]);
        }
    }
    for (i, l) in c.locations.iter().enumerate() {
        if let Some(parent) = &l.parent {
            ck.reference(parent, format!("locations[{i}].parent"), &[Kind::Location]);
        }
    }
    for (i, f) in c.factions.iter().enumerate() {
        for (j, r) in f.rivals.iter().enumerate() {
            ck.reference(r, format!("factions[{i}].rivals[{j}]"), &[Kind::Faction]);
        }
    }
    for (i, g) in c.goals.iter().enumerate() {
        if let Some(h) = &g.held_by {
            ck.reference(h, format!("goals[{i}].held_by"), &[Kind::Faction]);
        }
        if let Some(item) = &g.item {
            ck.reference(item, format!("goals[{i}].item"), &[Kind::Item]);
        }
    }
}

fn check_structure(ck: &mut Checker<'_>, c: &Campaign) {
    for (i, f) in c.fronts.iter().enumerate() {
        let n = f.steps.len();
        if !(FRONT_STEPS_MIN..=FRONT_STEPS_MAX).contains(&n) {
            ck.warn(
                "FRONT_CLOCK_LENGTH",
                format!("fronts[{i}].steps"),
                format!(
                    "`{}` has a {n}-step clock; a clock has {FRONT_STEPS_MIN} to {FRONT_STEPS_MAX} steps",
                    f.id
                ),
            );
        }
    }

    for (i, n) in c.nodes.iter().enumerate() {
        let p = format!("nodes[{i}]");
        for (j, x) in n.exits.iter().enumerate() {
            if x.to == n.id {
                ck.warn(
                    "EXIT_TO_SELF",
                    format!("{p}.exits[{j}]"),
                    format!("`{}` leads to itself", n.id),
                );
            }
        }
        for (j, l) in n.loot.iter().enumerate() {
            if l.item.is_none() && l.coins.unwrap_or(0) == 0 {
                ck.warn(
                    "LOOT_EMPTY",
                    format!("{p}.loot[{j}]"),
                    "loot with neither an item nor coins".into(),
                );
            }
        }
        if let Some(e) = &n.encounter {
            if e.opponents.is_empty() {
                ck.warn(
                    "ENCOUNTER_EMPTY",
                    format!("{p}.encounter.opponents"),
                    format!("`{}` holds a fight against nobody", n.id),
                );
            }
            for (j, o) in e.opponents.iter().enumerate() {
                if o.count == 0 {
                    ck.warn(
                        "ENCOUNTER_EMPTY",
                        format!("{p}.encounter.opponents[{j}].count"),
                        format!("zero `{}`", o.who),
                    );
                }
            }
        }
        check_music(ck, &n.ambience.music, &format!("{p}.ambience.music"));
    }
    for (i, a) in c.acts.iter().enumerate() {
        check_music(ck, &a.music, &format!("acts[{i}].music"));
    }

    for (i, f) in c.factions.iter().enumerate() {
        let a = f.affinity;
        if !(a.min <= a.start && a.start <= a.max) || a.min == a.max {
            ck.error(
                "AFFINITY_RANGE",
                format!("factions[{i}].affinity"),
                format!(
                    "`{}`: affinity needs min < max and start between them",
                    f.id
                ),
            );
        }
        for (j, r) in f.rivals.iter().enumerate() {
            if *r == f.id {
                ck.warn(
                    "FACTION_SELF_RIVAL",
                    format!("factions[{i}].rivals[{j}]"),
                    format!("`{}` is its own rival", f.id),
                );
            }
        }
    }

    // Reachability from the opening node.
    if c.nodes.is_empty() {
        return;
    }
    let Some(start) = c.bible.start_node.as_deref() else {
        ck.warn(
            "START_NODE_MISSING",
            "bible.start_node".into(),
            "no opening node".into(),
        );
        return;
    };
    if c.node(start).is_none() {
        return; // already a dangling reference
    }
    let exits: HashMap<&str, Vec<&str>> = c
        .nodes
        .iter()
        .map(|n| {
            (
                n.id.as_str(),
                n.exits.iter().map(|x| x.to.as_str()).collect(),
            )
        })
        .collect();
    let mut reached = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(cur) = queue.pop_front() {
        for next in exits.get(cur).into_iter().flatten() {
            if exits.contains_key(next) && reached.insert(next) {
                queue.push_back(next);
            }
        }
    }
    for (i, n) in c.nodes.iter().enumerate() {
        if !reached.contains(n.id.as_str()) {
            ck.warn(
                "NODE_UNREACHABLE",
                format!("nodes[{i}]"),
                format!(
                    "`{}` cannot be reached from the opening node `{start}`",
                    n.id
                ),
            );
        }
    }
}

fn check_music(ck: &mut Checker<'_>, tracks: &[MusicTrack], path: &str) {
    for (j, t) in tracks.iter().enumerate() {
        if t.url.is_empty() {
            ck.warn(
                "MUSIC_NO_URL",
                format!("{path}[{j}].url"),
                format!("`{}` has no link yet: a track to choose", t.title),
            );
        } else if !is_youtube(&t.url) {
            ck.warn(
                "MUSIC_NOT_YOUTUBE",
                format!("{path}[{j}].url"),
                format!("`{}` is not a YouTube link", t.url),
            );
        }
    }
}

/// Every party member has a reason to act somewhere in each act that
/// has scenes (`docs/lecons-des-parties.md` §2: six players, one head).
/// An act with no scene yet is unprepared, which is not this check's
/// concern.
fn check_player_hooks(ck: &mut Checker<'_>, c: &Campaign) {
    for (i, act) in c.acts.iter().enumerate() {
        let scenes: Vec<_> = c.nodes.iter().filter(|n| n.act == act.id).collect();
        if scenes.is_empty() {
            continue;
        }
        for p in &c.party {
            let hooked = scenes
                .iter()
                .any(|n| n.player_hooks.iter().any(|h| h.character == p.id));
            if !hooked {
                ck.warn(
                    "PLAYER_WITHOUT_HOOK",
                    format!("acts[{i}]"),
                    format!(
                        "`{}` has no player hook in any scene of act `{}`",
                        p.id, act.id
                    ),
                );
            }
        }
    }
}

/// Distinct nodes holding a clue of each revelation.
fn clue_nodes(c: &Campaign) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut by_rev: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for clue in &c.clues {
        by_rev
            .entry(clue.revelation.as_str())
            .or_default()
            .insert(clue.node.as_str());
    }
    by_rev
}

fn check_clues(ck: &mut Checker<'_>, c: &Campaign) {
    let by_rev = clue_nodes(c);
    for (i, r) in c.revelations.iter().enumerate() {
        let path = format!("revelations[{i}]");
        let clues = c.clues_for(&r.id).count();
        let nodes = by_rev.get(r.id.as_str()).map_or(0, BTreeSet::len);
        if clues == 0 {
            ck.warn(
                "REVELATION_NO_CLUE",
                path,
                format!("no clue leads to `{}`", r.id),
            );
        } else if r.importance == Importance::Critical && nodes < MIN_CRITICAL_CLUE_NODES {
            ck.warn(
                "THREE_CLUE_RULE",
                path,
                format!(
                    "critical revelation `{}` has {clues} clue(s) in {nodes} node(s); \
                     it needs clues in at least {MIN_CRITICAL_CLUE_NODES} different nodes",
                    r.id
                ),
            );
        }
    }
}

fn check_required_knowledge(ck: &mut Checker<'_>, c: &Campaign) {
    for (i, n) in c.nodes.iter().enumerate() {
        for (j, rev) in n.requires.iter().enumerate() {
            if c.revelations.iter().all(|r| r.id != *rev) {
                continue; // already a dangling reference
            }
            // Where else the players can learn it: not in this node,
            // which they are only entering.
            let elsewhere: Vec<&str> = c
                .clues_for(rev)
                .map(|cl| cl.node.as_str())
                .filter(|node| *node != n.id)
                .collect();
            let path = format!("nodes[{i}].requires[{j}]");
            if elsewhere.is_empty() {
                ck.warn(
                    "KNOWLEDGE_NEVER_GIVEN",
                    path,
                    format!(
                        "`{}` requires `{rev}`, but no other node gives a clue to it",
                        n.id
                    ),
                );
            } else if elsewhere
                .iter()
                .all(|node| c.node(node).is_some_and(|x| x.optional))
            {
                ck.warn(
                    "KNOWLEDGE_ONLY_OPTIONAL",
                    path,
                    format!(
                        "`{}` requires `{rev}`, which is only given in optional scenes",
                        n.id
                    ),
                );
            }
        }
    }
}
