//! The player view of a campaign: the **single projection point**
//! (`MEMORY.md` §3). Everything a player (or the shared screen) receives
//! about the campaign is built here, by copying out what players may
//! see; nothing is filtered on the client.
//!
//! It works by allow-list: a field reaches players only if this file
//! names it. Never shown: the bible's pitch, truths and secrets; fronts;
//! revelation statements; clues not yet found, and how any clue is
//! found; every scene but the current one, and in it the GM's summary,
//! flow, hook, planned checks, key points, tactics, morale, loot, XP,
//! transition, player hooks, fallback note and notes; what an NPC wants,
//! hides, their traits, flaw, motivation, stats and inventory; the name
//! of an NPC or adversary the players have not met, and any hit points
//! or stat block. Art fields are prompts for the image generator, not
//! player text.
//!
//! Every player and shared-screen route (`app::player_routes`,
//! `app::invitation_routes`) builds its answer with a function of this
//! file — [`project_invitation`], [`project_home`], [`project_for_players`]
//! — and `back/tests/player_routes_test.rs` sweeps them all for GM-only
//! markers. A grid map will reach players through `Map::project` with
//! `Viewer::Player` (`promptus_shared::maps`), called from here; the
//! character sheet is the player's own text (`players::CharacterSheet`)
//! and is the one document sent back whole.

use chrono::{DateTime, Utc};
use promptus_shared::story::{Campaign, MusicTrack, WorldState};
use serde::Serialize;
use uuid::Uuid;

use crate::players::{Character, CharacterSheet, CharacterStatus, Player, Role};

/// The label an unrevealed opponent goes by.
pub const UNKNOWN_OPPONENT: &str = "Adversaire";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerCampaignView {
    pub title: String,
    pub world: String,
    /// What the players were told when the campaign started.
    pub player_hook: String,
    pub party: Vec<PartyMemberView>,
    pub scene: Option<SceneView>,
    /// The text of every clue found, in the campaign's order.
    pub clues: Vec<String>,
    /// NPCs the players have met.
    pub npcs: Vec<NpcView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyMemberView {
    pub id: String,
    pub name: String,
    pub concept: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneView {
    pub id: String,
    pub title: String,
    pub read_aloud: String,
    pub place: Option<PlaceView>,
    pub music: Vec<MusicTrack>,
    /// Met NPCs present in the scene.
    pub npcs: Vec<NpcView>,
    /// The scene's opponents: a name only once revealed, never stats.
    pub opponents: Vec<OpponentView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceView {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcView {
    pub id: String,
    pub name: String,
    pub title: String,
    pub appearance: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpponentView {
    pub label: String,
    pub count: u32,
}

fn npc_view(campaign: &Campaign, id: &str) -> Option<NpcView> {
    campaign.npc(id).map(|n| NpcView {
        id: n.id.clone(),
        name: n.name.clone(),
        title: n.title.clone(),
        appearance: n.appearance.clone(),
    })
}

/// What players may see of `campaign` in its current `world`.
#[must_use]
pub fn project_for_players(campaign: &Campaign, world: &WorldState) -> PlayerCampaignView {
    let met = |id: &str| world.revealed.contains(id);

    let scene = world
        .current_node
        .as_deref()
        .and_then(|id| campaign.node(id))
        .map(|node| SceneView {
            id: node.id.clone(),
            title: node.title.clone(),
            read_aloud: node.read_aloud.clone(),
            place: node
                .location
                .as_deref()
                .and_then(|l| campaign.location(l))
                .map(|l| PlaceView {
                    name: l.name.clone(),
                    description: l.description.clone(),
                }),
            // A track still to choose (no link) is the GM's to-do, and
            // its search hint is GM text.
            music: node
                .ambience
                .music
                .iter()
                .filter(|t| !t.url.is_empty())
                .map(|t| MusicTrack {
                    search: String::new(),
                    ..t.clone()
                })
                .collect(),
            npcs: node
                .npcs
                .iter()
                .filter(|p| met(&p.npc))
                .filter_map(|p| npc_view(campaign, &p.npc))
                .collect(),
            opponents: node
                .encounter
                .iter()
                .flat_map(|e| &e.opponents)
                .map(|o| {
                    let name = campaign
                        .adversary(&o.who)
                        .map(|a| &a.name)
                        .or_else(|| campaign.npc(&o.who).map(|n| &n.name));
                    let label = match name {
                        Some(n) if met(&o.who) => n.clone(),
                        _ => UNKNOWN_OPPONENT.to_string(),
                    };
                    OpponentView {
                        label,
                        count: o.count,
                    }
                })
                .collect(),
        });

    PlayerCampaignView {
        title: campaign.title.clone(),
        world: campaign.world.clone(),
        player_hook: campaign.bible.player_hook.clone(),
        party: campaign
            .party
            .iter()
            .map(|p| PartyMemberView {
                id: p.id.clone(),
                name: p.name.clone(),
                concept: p.concept.clone(),
            })
            .collect(),
        scene,
        clues: campaign
            .clues
            .iter()
            .filter(|c| world.found_clues.contains(&c.id))
            .map(|c| c.text.clone())
            .collect(),
        npcs: campaign
            .npcs
            .iter()
            .filter(|n| met(&n.id))
            .filter_map(|n| npc_view(campaign, &n.id))
            .collect(),
    }
}

/// What an invitation link shows before joining: whose table, which
/// campaign, and the hook players were meant to read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationView {
    pub campaign_id: Uuid,
    pub title: String,
    pub world: String,
    pub player_hook: String,
    pub gm_name: String,
}

#[must_use]
pub fn project_invitation(campaign_id: Uuid, campaign: &Campaign, gm_name: &str) -> InvitationView {
    InvitationView {
        campaign_id,
        title: campaign.title.clone(),
        world: campaign.world.clone(),
        player_hook: campaign.bible.player_hook.clone(),
        gm_name: gm_name.to_string(),
    }
}

/// A player's home in a campaign: who they are at this table, the
/// campaign's header and their character.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerHomeView {
    pub me: MeView,
    pub campaign: InvitationView,
    pub character: Option<CharacterView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeView {
    pub id: Uuid,
    pub nickname: String,
    pub role: Role,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterView {
    pub id: Uuid,
    pub status: CharacterStatus,
    pub sheet: CharacterSheet,
    /// The GM's word when the sheet was returned; written for the player.
    pub gm_note: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[must_use]
pub fn project_home(
    campaign: &Campaign,
    gm_name: &str,
    player: &Player,
    character: Option<&Character>,
) -> PlayerHomeView {
    PlayerHomeView {
        me: MeView {
            id: player.id,
            nickname: player.nickname.clone(),
            role: player.role,
        },
        campaign: project_invitation(player.campaign_id, campaign, gm_name),
        character: character.map(|c| CharacterView {
            id: c.id,
            status: c.status,
            sheet: c.sheet.clone(),
            gm_note: c.gm_note.clone(),
            updated_at: c.updated_at,
        }),
    }
}
