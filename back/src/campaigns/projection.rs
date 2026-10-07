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
//! file — [`project_invitation`], [`project_home`], [`project_for_players`],
//! [`project_creation`] — and `back/tests/player_routes_test.rs` sweeps them all for GM-only
//! markers. A grid map will reach players through `Map::project` with
//! `Viewer::Player` (`promptus_shared::maps`), called from here; the
//! character sheet is the player's own text (`players::CharacterSheet`)
//! and is the one document sent back whole.
//!
//! The rule system reaches players the same way: names, descriptions,
//! scores and action cards, never the GM's notes on a class, the
//! creation rule, an item or an action tag ([`project_creation`],
//! [`project_play`]). The rules page is [`rules::project_rules`].
//!
//! A validated character's play state (`players::play`) reaches its
//! player through [`project_play`]: hit points, XP and level, resources
//! and the bag, all derived through the rules engine. The history of
//! adjustments stays GM-side.

pub mod board;
pub mod evening;
pub mod rules;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::action::{ActionCard, action_cards};
use promptus_shared::rules::level_up::{
    HitPointGain, cards_unlocked, hit_point_options, hit_points_due,
};
use promptus_shared::rules::model::{ActionDef, RollSpec, Tag};
use promptus_shared::rules::sheet::Combatant;
use promptus_shared::sprite::CharacterLook;
use promptus_shared::story::{Campaign, MusicTrack, WorldState};
use serde::Serialize;
use uuid::Uuid;

use crate::players::fate::{Fallen, Next};
use crate::players::play::{PlayState, combatant};
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
    /// The act's id: its introduction video, once approved, plays here.
    pub act: String,
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
            act: node.act.clone(),
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
    /// The living character, if any.
    pub character: Option<CharacterView>,
    /// The player's last dead character (player/face-death): their last
    /// words and what the player chose next.
    pub fallen: Option<FallenView>,
}

/// A dead character as its player reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallenView {
    pub character_id: Uuid,
    pub name: String,
    pub class_name: Option<String>,
    pub look: Option<CharacterLook>,
    pub level: u32,
    pub last_words: Option<String>,
    pub next: Option<Next>,
    pub died_at: DateTime<Utc>,
}

/// `fallen` as its player reads it.
#[must_use]
pub fn project_fallen(rules: Option<&RuleSystem>, fallen: &Fallen) -> FallenView {
    FallenView {
        character_id: fallen.character_id,
        name: fallen.name.clone(),
        class_name: fallen
            .class_id
            .as_deref()
            .and_then(|id| rules?.class(id))
            .map(|c| c.name.clone()),
        look: fallen.look.clone(),
        level: fallen.level,
        last_words: fallen.last_words.clone(),
        next: fallen.next,
        died_at: fallen.died_at,
    }
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
    /// The names of the sheet's people and class in the rules, so the
    /// sheet reads « nain · guerrier » without the whole rule system.
    pub people_name: Option<String>,
    pub class_name: Option<String>,
    /// What the rules make of the sheet, once it has a class.
    pub stats: Option<SheetStatsView>,
    /// The character in play, once validated and with a class of the
    /// rules.
    pub play: Option<PlayView>,
}

/// A character in play, as its player and the GM read it: what the
/// server holds (`players::play::PlayState`) and what the rules derive
/// from it and the sheet. Nothing GM-only.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayView {
    pub level: u32,
    pub total_xp: u32,
    /// The bar that turns into an upgrade point when full.
    pub xp_bar: u32,
    pub xp_bar_max: u32,
    /// Upgrade points earned and not spent yet.
    pub upgrade_points: u32,
    /// Total XP the next level asks for; `None` at the last one.
    pub next_level_xp: Option<u32>,
    pub hit_points: i32,
    pub max_hit_points: i32,
    pub armor_class: i32,
    pub initiative: i32,
    pub abilities: Vec<AbilityScoreView>,
    /// Every card of the class at this level; `level` above the
    /// character's says it is still locked.
    pub cards: Vec<ActionCardView>,
    pub resources: Vec<ResourceView>,
    pub inventory: Vec<ItemView>,
    /// A level reached that the player has not gone through yet
    /// (engine/level-up).
    pub level_up: Option<LevelUpView>,
}

/// What the new levels bring, from the last one the player went through.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelUpView {
    pub from: u32,
    pub to: u32,
    /// The die-or-average choice, when the rules add hit points per level.
    pub hit_points: Option<LevelHitPointsChoice>,
    /// What the levels since `from` added, as taken.
    pub gains: Vec<HitPointGain>,
    /// The class cards these levels unlock.
    pub cards: Vec<ActionCardView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelHitPointsChoice {
    pub dice: String,
    pub average: i32,
    /// The ability's name in the rules (« Constitution »).
    pub ability: Option<String>,
    pub modifier: i32,
    /// The levels whose hit points are still to take.
    pub due: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilityScoreView {
    pub id: String,
    pub name: String,
    pub score: i32,
    pub modifier: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceView {
    pub id: String,
    pub name: String,
    pub abbr: String,
    pub amount: i32,
}

/// A bag line: the rule system's name and description for its items
/// (never its GM note), the GM's words for an item they named.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub key: String,
    pub item_id: Option<String>,
    pub name: String,
    pub description: String,
    pub qty: u32,
    pub consumable: bool,
    pub equipped: bool,
}

/// `sheet` in play with `state` (its starting state when `None`), under
/// `rules`. `None` when the sheet names no class of the rules.
#[must_use]
pub fn project_play(
    rules: &RuleSystem,
    sheet: &CharacterSheet,
    state: Option<&PlayState>,
) -> Option<PlayView> {
    let start;
    let state = match state {
        Some(s) => s,
        None => {
            start = PlayState::start(rules, sheet);
            &start
        }
    };
    let c = combatant(rules, sheet, state)?;
    let progress = c.progress.unwrap_or_default();
    let level = c.level(rules).unwrap_or(1);
    let level_up = (level > state.level_seen).then(|| {
        let from = state.level_seen;
        let unlocked: Vec<&str> = cards_unlocked(&c, rules, from, level)
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        LevelUpView {
            from,
            to: level,
            hit_points: hit_point_options(rules, &c)
                .ok()
                .flatten()
                .map(|o| LevelHitPointsChoice {
                    dice: o.dice,
                    average: o.average,
                    ability: o.ability.map(|a| {
                        rules
                            .ability(&a)
                            .map_or_else(|| a.clone(), |d| d.name.clone())
                    }),
                    modifier: o.modifier,
                    due: hit_points_due(rules, state.total_xp, &state.chosen_levels()),
                }),
            gains: state
                .hit_point_gains
                .iter()
                .filter(|g| g.level > from && g.level <= level)
                .cloned()
                .collect(),
            cards: action_cards(rules, &c)
                .iter()
                .filter(|card| unlocked.contains(&card.action.id.as_str()))
                .map(|card| card_view(rules, card))
                .collect(),
        }
    });
    Some(PlayView {
        level_up,
        level,
        total_xp: progress.total_xp,
        xp_bar: progress.bar,
        xp_bar_max: rules.progression.upgrade_every_xp,
        upgrade_points: progress.upgrade_points,
        next_level_xp: rules
            .progression
            .levels
            .iter()
            .filter(|l| l.level > level)
            .map(|l| l.xp)
            .min(),
        hit_points: c.hit_points,
        max_hit_points: c.max_hit_points(rules).ok()?,
        armor_class: c.armor_class(rules).ok()?,
        initiative: c.initiative_bonus(rules).ok()?,
        abilities: rules
            .abilities
            .iter()
            .filter_map(|a| {
                Some(AbilityScoreView {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    score: c.score(&a.id)?,
                    modifier: c.modifier(rules, &a.id).ok()?,
                })
            })
            .collect(),
        cards: action_cards(rules, &c)
            .iter()
            .map(|card| card_view(rules, card))
            .collect(),
        resources: rules
            .resources
            .iter()
            .map(|r| ResourceView {
                id: r.id.clone(),
                name: r.name.clone(),
                abbr: r.abbr.clone(),
                amount: state.resource(rules, &r.id),
            })
            .collect(),
        inventory: state
            .inventory
            .iter()
            .map(|e| {
                let def = e.item.as_deref().and_then(|id| rules.item(id));
                ItemView {
                    key: e.key.clone(),
                    item_id: e.item.clone(),
                    name: e.display_name(rules),
                    description: def
                        .map_or_else(|| e.description.clone(), |d| d.description.clone()),
                    qty: e.qty,
                    consumable: def.is_some_and(|d| d.consumable),
                    equipped: e.equipped,
                }
            })
            .collect(),
    })
}

/// A sheet as the rules read it at level 1: the numbers the summary
/// shows, computed by the server (`MEMORY.md` §3).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetStatsView {
    pub hit_points: i32,
    pub armor_class: i32,
    pub initiative: i32,
    /// Modifier by ability id.
    pub modifiers: BTreeMap<String, i32>,
    /// The class's action cards, attack bonuses from this sheet's scores.
    pub cards: Vec<ActionCardView>,
}

/// What a player's action card shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionCardView {
    pub id: String,
    pub name: String,
    pub description: String,
    /// The name of the action kind it spends (« Attaque »).
    pub kind: String,
    /// The level it unlocks at; 1 for a starting card.
    pub level: u32,
    /// Added to the attack roll, for actions that roll to hit.
    pub attack_bonus: Option<i32>,
    /// Damage dealt, as dice (« 3 », « 1d6 »).
    pub damage: Option<String>,
    /// Hit points healed, as dice.
    pub heal: Option<String>,
    /// Turns before it can be played again.
    pub cooldown: u32,
    /// Reach in cells.
    pub range: u32,
}

fn card_view(system: &RuleSystem, card: &ActionCard<'_>) -> ActionCardView {
    action_view(system, card.action, card.attack_bonus)
}

fn action_view(system: &RuleSystem, a: &ActionDef, attack_bonus: Option<i32>) -> ActionCardView {
    ActionCardView {
        id: a.id.clone(),
        name: a.name.clone(),
        description: a.description.clone(),
        kind: system
            .action_kind(&a.kind)
            .map_or_else(|| a.kind.clone(), |k| k.name.clone()),
        level: a.level.unwrap_or(1),
        attack_bonus: if a.roll == RollSpec::Attack {
            attack_bonus
        } else {
            None
        },
        damage: a.tags.iter().find_map(|t| match t {
            Tag::Damage(d) => Some(d.amount.to_string()),
            _ => None,
        }),
        heal: a.tags.iter().find_map(|t| match t {
            Tag::Heal(h) => Some(h.amount.to_string()),
            _ => None,
        }),
        cooldown: a.cooldown(),
        range: a.reach(),
    }
}

/// A level-1 combatant of `class_id` with `scores` over the class's own.
fn level_one(
    system: &RuleSystem,
    class_id: &str,
    scores: &BTreeMap<String, i32>,
) -> Option<Combatant> {
    let mut c = Combatant::from_class(system, "sheet", "", class_id).ok()?;
    for (id, score) in scores {
        if c.abilities.contains_key(id) {
            c.abilities.insert(id.clone(), *score);
        }
    }
    c.hit_points = c.max_hit_points(system).ok()?;
    Some(c)
}

fn stats_view(system: &RuleSystem, c: &Combatant) -> Option<SheetStatsView> {
    Some(SheetStatsView {
        hit_points: c.hit_points,
        armor_class: c.armor_class(system).ok()?,
        initiative: c.initiative_bonus(system).ok()?,
        modifiers: system
            .abilities
            .iter()
            .filter_map(|a| Some((a.id.clone(), c.modifier(system, &a.id).ok()?)))
            .collect(),
        cards: action_cards(system, c)
            .iter()
            .map(|card| card_view(system, card))
            .collect(),
    })
}

/// The numbers of `sheet` under `rules`, once it names a class.
#[must_use]
pub fn sheet_stats(rules: Option<&RuleSystem>, sheet: &CharacterSheet) -> Option<SheetStatsView> {
    let rules = rules?;
    let c = level_one(rules, sheet.class_id.as_deref()?, &sheet.abilities)?;
    stats_view(rules, &c)
}

/// A player's own character: their sheet whole (it holds nothing
/// GM-only), its status, the GM's note to them, and its numbers.
#[must_use]
pub fn project_character(rules: Option<&RuleSystem>, c: &Character) -> CharacterView {
    CharacterView {
        id: c.id,
        status: c.status,
        sheet: c.sheet.clone(),
        gm_note: c.gm_note.clone(),
        updated_at: c.updated_at,
        people_name: rules.and_then(|r| {
            let id = c.sheet.people_id.as_deref()?;
            r.peoples
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
        }),
        class_name: rules.and_then(|r| Some(r.class(c.sheet.class_id.as_deref()?)?.name.clone())),
        stats: sheet_stats(rules, &c.sheet),
        play: match (rules, c.status) {
            (Some(rules), CharacterStatus::Validated) => {
                project_play(rules, &c.sheet, c.play.as_ref())
            }
            _ => None,
        },
    }
}

#[must_use]
pub fn project_home(
    campaign: &Campaign,
    gm_name: &str,
    player: &Player,
    character: Option<&Character>,
    fallen: Option<&Fallen>,
    rules: Option<&RuleSystem>,
) -> PlayerHomeView {
    PlayerHomeView {
        me: MeView {
            id: player.id,
            nickname: player.nickname.clone(),
            role: player.role,
        },
        campaign: project_invitation(player.campaign_id, campaign, gm_name),
        character: character.map(|c| project_character(rules, c)),
        fallen: fallen.map(|f| project_fallen(rules, f)),
    }
}

/// What the character creator needs (`characters/build-character-creator`):
/// the sprite pack to pick pieces from, the look a new character starts
/// with, and what the rule system asks of a character.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationView {
    /// The sprite pack id (`GET /api/sprites/packs/{pack}` lists it).
    pub pack: String,
    pub start_look: CharacterLook,
    /// `None` when the server does not have the campaign's rule system:
    /// the creator then asks for a look, a name and a story only.
    pub rules: Option<CreationRulesView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationRulesView {
    pub name: String,
    pub abilities: Vec<NamedView>,
    /// Empty when the system has no peoples: the step is skipped.
    pub peoples: Vec<NamedView>,
    pub classes: Vec<ClassView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedView {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub primary_abilities: Vec<String>,
    /// The class's scores: the suggested spread the creator starts from.
    pub abilities: BTreeMap<String, i32>,
    /// Points the class's spread adds up to. Going over is flagged to
    /// the player and settled by the GM, never refused.
    pub budget: i32,
    /// The class's sheet with its own scores.
    pub stats: Option<SheetStatsView>,
}

#[must_use]
pub fn project_creation(
    pack: &str,
    start_look: CharacterLook,
    rules: Option<&RuleSystem>,
) -> CreationView {
    let named = |id: &str, name: &str, description: &str| NamedView {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
    };
    CreationView {
        pack: pack.to_string(),
        start_look,
        rules: rules.map(|r| CreationRulesView {
            name: r.name.clone(),
            abilities: r
                .abilities
                .iter()
                .map(|a| named(&a.id, &a.name, &a.description))
                .collect(),
            peoples: r
                .peoples
                .iter()
                .map(|p| named(&p.id, &p.name, &p.description))
                .collect(),
            classes: r
                .classes
                .iter()
                .map(|c| ClassView {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    description: c.description.clone(),
                    primary_abilities: c.primary_abilities.clone(),
                    abilities: c.abilities.clone(),
                    budget: c.abilities.values().sum(),
                    stats: level_one(r, &c.id, &BTreeMap::new()).and_then(|lv| stats_view(r, &lv)),
                })
                .collect(),
        }),
    }
}
