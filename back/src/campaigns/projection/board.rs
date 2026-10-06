//! The grid as a player (or the shared screen) receives it, built here
//! by allow-list (`MEMORY.md` §3, maps/reveal-fog-and-hidden).
//!
//! Reaches players: the map cut by `Map::project(Player)` then, with the
//! fog on, `Map::fogged` (nothing of a fogged cell: not its material,
//! not a door, prop, object, light, label, start or exit there); the
//! tokens standing on revealed cells and not hidden by the GM — an
//! invisible one only on its owner's screen, as a ghost; in a fight,
//! the order, whose turn, each combatant's standing, the party's hit
//! points (an opponent's never: only whether it is down), the caller's
//! cards and reachable cells on their turn, the events with opponents'
//! hit points and the GM's notes removed, and the loot already given.
//!
//! Never: GM layers, GM notes, object checks, the backdrop prompt,
//! fogged cells, hidden tokens, opponents' hit points, the co-GM's
//! proposal, loot not handed out yet.

use std::collections::BTreeSet;

use promptus_shared::combat::fight::{Fight, FightEvent, Standing};
use promptus_shared::maps::{Cell, Map, Viewer};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::action::action_cards;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::model::{AreaShape, Targeting};
use promptus_shared::rules::sheet::Side;
use serde::Serialize;
use uuid::Uuid;

use crate::board::fight::{Encounter, LootLine};
use crate::board::{Board, TokenKind};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    pub map: Map,
    pub fog: bool,
    pub tokens: Vec<TokenView>,
    /// Where the caller's token may walk now, out of a fight.
    pub reachable: Vec<ReachCell>,
    pub fight: Option<FightView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenView {
    pub id: String,
    pub name: String,
    pub at: Cell,
    pub party: bool,
    pub mine: bool,
    /// Invisible: only its owner sees it, as a ghost.
    pub ghost: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReachCell {
    pub at: Cell,
    pub cost: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightView {
    pub live: bool,
    pub round: u32,
    pub active: Option<String>,
    pub my_turn: bool,
    pub order: Vec<FighterView>,
    /// The caller's actions left this turn, on their turn.
    pub actions_left: Option<u32>,
    pub cards: Vec<FightCardView>,
    pub events: Vec<FightEvent>,
    /// What the party received, line by line.
    pub loot: Vec<GivenLoot>,
    pub won: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FighterView {
    pub id: String,
    pub name: String,
    pub party: bool,
    pub standing: Standing,
    pub at: Option<Cell>,
    /// Party members only.
    pub hit_points: Option<i32>,
    pub max_hit_points: Option<i32>,
    pub down: bool,
    pub conditions: Vec<String>,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightCardView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub target: Targeting,
    pub area: Option<AreaShape>,
    pub range: u32,
    pub attack_bonus: Option<i32>,
    /// Why it cannot be played now, as the engine says it.
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GivenLoot {
    pub name: String,
    pub to_me: bool,
}

/// Who is looking.
pub struct Looker<'a> {
    /// The caller's character, when they have one validated.
    pub character: Option<Uuid>,
    pub rules: Option<&'a RuleSystem>,
}

fn token_of(character: Option<Uuid>) -> Option<String> {
    character.map(crate::board::character_token)
}

/// An event as a player may read it.
fn event_for_players(
    fight: &Fight,
    e: &FightEvent,
    seen: &dyn Fn(Cell) -> bool,
) -> Option<FightEvent> {
    let opponent = |id: &str| {
        fight
            .combatant(id)
            .is_some_and(|c| c.side == Side::Opposition)
    };
    match e {
        // An opponent's walk shows only where the party sees it.
        FightEvent::Moved {
            who,
            path,
            cost,
            budget,
        } if opponent(who) => {
            let path: Vec<Cell> = path.iter().copied().filter(|c| seen(*c)).collect();
            (!path.is_empty()).then(|| FightEvent::Moved {
                who: who.clone(),
                path,
                cost: *cost,
                budget: *budget,
            })
        }
        FightEvent::Rules { event } => match event {
            Event::ForTheGm { .. } => None,
            Event::Damaged {
                target, breakdown, ..
            } if opponent(target) => Some(FightEvent::Rules {
                event: Event::Damaged {
                    target: target.clone(),
                    breakdown: breakdown.clone(),
                    hp_before: 0,
                    hp_after: 0,
                },
            }),
            Event::Healed { target, amount, .. } if opponent(target) => Some(FightEvent::Rules {
                event: Event::Healed {
                    target: target.clone(),
                    amount: *amount,
                    hp_before: 0,
                    hp_after: 0,
                },
            }),
            _ => Some(e.clone()),
        },
        _ => Some(e.clone()),
    }
}

fn fight_view(
    enc: &Encounter,
    events: &[FightEvent],
    looker: &Looker<'_>,
    seen: &dyn Fn(Cell) -> bool,
) -> FightView {
    let f = &enc.fight;
    let mine = token_of(looker.character);
    let my_turn = enc.live && mine.as_deref().is_some_and(|m| f.active() == Some(m));
    let order = f
        .order
        .iter()
        .filter_map(|id| {
            let c = f.combatant(id)?;
            let party = c.side == Side::Party;
            let at = f.position(id).filter(|at| party || seen(*at));
            // An opponent the party does not see is not in the list.
            if !party && at.is_none() && f.standing.get(id) == Some(&Standing::InFight) {
                return None;
            }
            let max = looker.rules.and_then(|r| c.max_hit_points(r).ok());
            Some(FighterView {
                id: id.clone(),
                name: c.name.clone(),
                party,
                standing: crate::board::fight::standing(f, id),
                at,
                hit_points: party.then_some(c.hit_points),
                max_hit_points: if party { max } else { None },
                down: c.hit_points <= 0,
                conditions: c.conditions.iter().map(|x| x.name.clone()).collect(),
                mine: mine.as_deref() == Some(id.as_str()),
            })
        })
        .collect();
    let (actions_left, cards) = match (
        my_turn,
        looker.rules,
        mine.as_deref().and_then(|m| f.combatant(m)),
    ) {
        (true, Some(r), Some(me)) => (
            Some(me.turn.actions_left),
            action_cards(r, me)
                .into_iter()
                .map(|c| FightCardView {
                    id: c.action.id.clone(),
                    name: c.action.name.clone(),
                    description: c.action.description.clone(),
                    kind: c.action.kind.clone(),
                    target: c.action.target,
                    area: c.action.area,
                    range: c.action.reach(),
                    attack_bonus: c.attack_bonus,
                    locked: c.locked.is_some(),
                })
                .collect(),
        ),
        _ => (None, Vec::new()),
    };
    FightView {
        live: enc.live,
        round: f.round,
        active: f.active().map(str::to_string),
        my_turn,
        order,
        actions_left,
        cards,
        events: events
            .iter()
            .filter_map(|e| event_for_players(f, e, seen))
            .collect(),
        loot: enc
            .loot
            .iter()
            .filter_map(|l: &LootLine| {
                l.given_to.map(|to| GivenLoot {
                    name: l.name.clone(),
                    to_me: Some(to) == looker.character,
                })
            })
            .collect(),
        won: f.end.as_ref().map(|e| e.winner == Some(Side::Party)),
    }
}

/// The grid as `looker` may see it.
#[must_use]
pub fn project_board(
    board: &Board,
    encounter: Option<(&Encounter, &[FightEvent])>,
    looker: &Looker<'_>,
) -> BoardView {
    let all: BTreeSet<Cell> = board.map.grid.cells().collect();
    let revealed = if board.fog { &board.revealed } else { &all };
    let seen = |c: Cell| revealed.contains(&c);
    let projected = board.map.project(Viewer::Player);
    let map = if board.fog {
        projected.fogged(revealed)
    } else {
        projected
    };
    let mine = token_of(looker.character);
    let tokens = board
        .tokens
        .iter()
        .filter(|t| !t.hidden && seen(t.at))
        .filter(|t| !t.invisible || mine.as_deref() == Some(t.id.as_str()))
        .map(|t| TokenView {
            id: t.id.clone(),
            name: t.name.clone(),
            at: t.at,
            party: t.kind == TokenKind::Character,
            mine: mine.as_deref() == Some(t.id.as_str()),
            ghost: t.invisible,
        })
        .collect();
    let in_fight = encounter.is_some_and(|(e, _)| e.live);
    let reachable = match (&mine, in_fight) {
        (Some(m), false) => board
            .reachable(looker.rules, m)
            .into_iter()
            .map(|(at, cost)| ReachCell { at, cost })
            .collect(),
        (Some(m), true) => match (encounter, looker.rules) {
            (Some((e, _)), Some(r)) => crate::board::fight::reachable(r, &e.fight, m)
                .into_iter()
                .map(|(at, cost)| ReachCell { at, cost })
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    BoardView {
        map,
        fog: board.fog,
        tokens,
        reachable,
        fight: encounter.map(|(e, ev)| fight_view(e, ev, looker, &seen)),
    }
}
