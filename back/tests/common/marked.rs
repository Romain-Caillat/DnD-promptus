//! The Phare de Kerbrume fixture with every GM-only field overwritten
//! by a marker naming it (`GMONLY<field>`), and a world that shows the
//! most. A player-facing answer built from them must carry no marker
//! (`projection_test.rs`, `player_routes_test.rs`).

use promptus_shared::story::{Campaign, MusicMood, MusicTrack, WorldState, from_yaml};

pub const FIXTURE: &str = include_str!("../../../content/fixtures/phare-de-kerbrume.yaml");
/// Hit points no text of the fixture contains.
pub const SECRET_HP: i32 = 4_271;

/// The GM's note on the Corsaires' `epee_de_bonne_facture`.
pub const ITEM_NOTE: &str = "l'épée standard n'est définie nulle part";

pub fn m(field: &str) -> String {
    format!("GMONLY<{field}>")
}

/// The fixture with every GM-only field marked.
pub fn marked() -> Campaign {
    let mut c = from_yaml(FIXTURE).unwrap();
    let b = &mut c.bible;
    b.pitch = m("bible.pitch");
    b.tone = m("bible.tone");
    b.themes = vec![m("bible.themes")];
    b.art_direction = m("bible.art_direction");
    b.truths = vec![m("bible.truths")];
    b.secrets = vec![m("bible.secrets")];
    for a in &mut c.acts {
        a.summary = m("acts.summary");
        a.gm_notes = m("acts.gm_notes");
    }
    for f in &mut c.fronts {
        f.name = m("fronts.name");
        f.goal = m("fronts.goal");
        f.description = m("fronts.description");
        for s in &mut f.steps {
            s.label = m("fronts.steps.label");
            s.description = m("fronts.steps.description");
        }
    }
    for r in &mut c.revelations {
        r.statement = m("revelations.statement");
    }
    for cl in &mut c.clues {
        cl.discovery = m("clues.discovery");
        if !["cl_gwen", "cl_carte"].contains(&cl.id.as_str()) {
            cl.text = m("clues.text (not found)");
        }
    }
    for n in &mut c.nodes {
        n.summary = m("nodes.summary");
        n.flow = m("nodes.flow");
        n.hook = m("nodes.hook");
        n.ambience.mood = m("nodes.ambience.mood");
        n.ambience.sounds = m("nodes.ambience.sounds");
        for t in &mut n.ambience.music {
            t.search = m("nodes.ambience.music.search");
        }
        // A track still to choose: no link, so nothing to play.
        n.ambience.music.push(MusicTrack {
            mood: MusicMood::Calm,
            title: m("nodes.ambience.music (no url)"),
            url: String::new(),
            search: m("nodes.ambience.music.search"),
        });
        n.map = Some(m("nodes.map"));
        for k in &mut n.checks {
            k.action = m("nodes.checks.action");
            k.success = m("nodes.checks.success");
            k.failure = m("nodes.checks.failure");
            k.natural_1 = m("nodes.checks.natural_1");
            k.natural_20 = m("nodes.checks.natural_20");
        }
        for p in &mut n.npcs {
            p.role = m("nodes.npcs.role");
        }
        n.key_points = vec![m("nodes.key_points")];
        if let Some(e) = &mut n.encounter {
            e.tactics = vec![m("nodes.encounter.tactics")];
            for r in &mut e.morale {
                r.when = m("nodes.encounter.morale.when");
                r.then = m("nodes.encounter.morale.then");
            }
            e.on_victory = m("nodes.encounter.on_victory");
            e.on_defeat = m("nodes.encounter.on_defeat");
        }
        for l in &mut n.loot {
            l.found = m("nodes.loot.found");
        }
        for x in &mut n.xp {
            x.reason = m("nodes.xp.reason");
        }
        n.transition = m("nodes.transition");
        for x in &mut n.exits {
            x.label = m("nodes.exits.label");
        }
        for h in &mut n.player_hooks {
            h.reason = m("nodes.player_hooks.reason");
        }
        n.if_skipped = m("nodes.if_skipped");
        n.art = m("nodes.art");
        n.gm_notes = m("nodes.gm_notes");
        if n.id != "sc_crique" {
            n.title = m("nodes.title (other scene)");
            n.read_aloud = m("nodes.read_aloud (other scene)");
        }
    }
    for p in &mut c.party {
        p.class = Some(m("party.class"));
    }
    for n in &mut c.npcs {
        n.portrait = m("npcs.portrait");
        n.roleplay = m("npcs.roleplay");
        n.traits = vec![m("npcs.traits")];
        n.flaw = m("npcs.flaw");
        n.motivation = m("npcs.motivation");
        n.wants = m("npcs.wants");
        n.hides = m("npcs.hides");
        n.age = m("npcs.age");
        n.gm_notes = m("npcs.gm_notes");
        if let Some(s) = &mut n.stats {
            s.hit_points = Some(SECRET_HP);
            for a in &mut s.attacks {
                a.name = m("npcs.stats.attacks");
            }
        }
        if n.id == "pnj_corentin" {
            // Not met: even his name is secret.
            n.name = m("npcs.name (not met)");
            n.title = m("npcs.title (not met)");
            n.appearance = m("npcs.appearance (not met)");
        }
    }
    for a in &mut c.adversaries {
        a.name = m("adversaries.name (not revealed)");
        a.description = m("adversaries.description");
        a.art = m("adversaries.art");
        a.gm_notes = m("adversaries.gm_notes");
        a.stats.hit_points = Some(SECRET_HP);
        for at in &mut a.stats.attacks {
            at.name = m("adversaries.stats.attacks");
        }
    }
    for l in &mut c.locations {
        l.art = m("locations.art");
        l.gm_notes = m("locations.gm_notes");
    }
    for i in &mut c.items {
        i.name = m("items.name");
        i.description = m("items.description");
        i.effect = m("items.effect");
        i.art = m("items.art");
        i.gm_notes = m("items.gm_notes");
    }
    for f in &mut c.factions {
        f.diplomacy = m("factions.diplomacy");
        f.art = m("factions.art");
        f.gm_notes = m("factions.gm_notes");
        if f.id != "fac_douane" {
            // Not known to the table: even its name is secret.
            f.name = m("factions.name (unknown)");
            f.description = m("factions.description (unknown)");
        }
    }
    for g in &mut c.goals {
        g.gm_notes = m("goals.gm_notes");
    }
    // A second goal the table has not heard of.
    c.goals.push(promptus_shared::story::Goal {
        id: "but_secret".into(),
        title: m("goals.title (unknown)"),
        description: m("goals.description (unknown)"),
        held_by: Some("fac_contrebandiers".into()),
        item: None,
        gm_notes: m("goals.gm_notes"),
    });
    c
}

/// In the crique, Gwen's and the map's clues found, Loïc met; the
/// douane known and in favour (the smugglers' gauge dropped out of
/// sight), the lantern to bring back known.
pub fn world(c: &Campaign) -> WorldState {
    let mut w = WorldState::default();
    w.enter_node(c, "sc_taverne").unwrap();
    w.enter_node(c, "sc_crique").unwrap();
    w.reveal_clue(c, "cl_gwen").unwrap();
    w.reveal_clue(c, "cl_carte").unwrap();
    w.reveal_entity(c, "pnj_loic").unwrap();
    w.shift_affinity(c, "fac_douane", 2).unwrap();
    w.set_goal(
        c,
        "but_lumiere",
        Some(promptus_shared::story::GoalStatus::Known),
    )
    .unwrap();
    w
}

/// The GM-only markers and secret hit points found in `json`.
pub fn leaks(json: &str) -> Vec<String> {
    let mut out: Vec<String> = json
        .match_indices("GMONLY<")
        .map(|(i, _)| json[i..json.len().min(i + 60)].to_string())
        .collect();
    if json.contains(&SECRET_HP.to_string()) {
        out.push(format!("hit points {SECRET_HP}"));
    }
    out
}

/// Give Marc's character (`character` of `campaign`) everything the GM
/// keeps about a sheet, each marked: the snapshot of the last review,
/// a secret hook drawn from the backstory (`players::review`) and a line
/// of the history of play adjustments (`players::play`). A player route
/// must show none of it. His bag holds a rules item whose GM note
/// ([`ITEM_NOTE`]) must not reach him either.
pub async fn mark_review(pool: &sqlx::PgPool, campaign: uuid::Uuid, character: uuid::Uuid) {
    sqlx::query("UPDATE characters SET reviewed_sheet = $2, reviewed_at = now() WHERE id = $1")
        .bind(character)
        .bind(serde_json::json!({ "name": m("characters.reviewed_sheet") }))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO secret_hooks (campaign_id, character_id, title, body, links)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(campaign)
    .bind(character)
    .bind(m("secret_hooks.title"))
    .bind(m("secret_hooks.body"))
    .bind(serde_json::json!([m("secret_hooks.links")]))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO character_play (character_id, campaign_id, inventory) VALUES ($1, $2, $3)",
    )
    .bind(character)
    .bind(campaign)
    .bind(serde_json::json!([
        { "key": "k1", "item": "epee_de_bonne_facture", "qty": 1 }
    ]))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO play_adjustments
           (campaign_id, character_id, actor, kind, label, before_value, after_value)
         VALUES ($1, $2, 'gm', 'item', $3, 0, 1)",
    )
    .bind(campaign)
    .bind(character)
    .bind(m("play_adjustments.label"))
    .execute(pool)
    .await
    .unwrap();
}
