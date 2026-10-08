//! A ship battle as a player receives it, built here by allow-list
//! (`MEMORY.md` §3, engine/support-vehicle-combat).
//!
//! Reaches players: the battle map cut by `Map::project(Player)`; every
//! ship's place, bow and standing; the party ship's gauges, power and
//! damages in full; an enemy ship's gauges only once the sensors (or the
//! lookout) scanned it; the crew and their stations; the caller's own
//! actions left and, for each action of their station, what it can aim
//! at now (targets in arc and range, cells within a manoeuvre, damages to
//! fix), computed by the engine; the events with an unscanned enemy's
//! gauges removed; how the battle ended.
//!
//! Never: an unscanned enemy's hull, screen, armour or morale, the
//! co-GM's proposal, the enemy ships' tactics.

use promptus_shared::maps::{Cell, Direction, Map, Viewer};
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::sheet::Side;
use promptus_shared::vehicle::battle::BattleEndReason;
use promptus_shared::vehicle::model::VehicleEffect;
use promptus_shared::vehicle::{Battle, BattleEvent, Facing, Ship, ShipStanding};
use serde::Serialize;

use crate::board::battle::StoredBattle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleView {
    /// Not over yet (a boarding included).
    pub live: bool,
    /// The fight is on a deck: the battle waits for it.
    pub boarding: bool,
    pub round: u32,
    /// The party ship's turn: the whole crew acts.
    pub crew_turn: bool,
    /// The ship or squad whose turn it is.
    pub active: Option<String>,
    pub map: Map,
    /// Where the wind (or the pull) comes from.
    pub current: Option<Direction>,
    pub gauges: Gauges,
    pub ships: Vec<ShipView>,
    pub crew: Vec<CrewView>,
    pub stations: Vec<StationView>,
    pub power: Option<PowerView>,
    pub me: Option<MeView>,
    pub events: Vec<BattleEvent>,
    pub won: Option<bool>,
    pub reason: Option<BattleEndReason>,
}

/// What the rules call each gauge (« Coque », « Voilure »…).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauges {
    pub hull: String,
    pub screen: Option<String>,
    pub armor: String,
    pub morale: String,
    pub power: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShipView {
    pub id: String,
    pub name: String,
    pub party: bool,
    pub at: Cell,
    pub facing: Facing,
    pub standing: ShipStanding,
    /// Its numbers are known to the crew (the party ship's always are).
    pub known: bool,
    pub hull: Option<i32>,
    pub max_hull: Option<i32>,
    pub screen: Option<i32>,
    pub max_screen: Option<i32>,
    pub armor: Option<i32>,
    pub morale: Option<i32>,
    pub max_morale: Option<i32>,
    pub damages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrewView {
    pub id: String,
    pub name: String,
    pub station: Option<String>,
    /// Played by the GM (the ship's holder).
    pub npc: bool,
    pub done: bool,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StationView {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Who sits there.
    pub holder: Option<String>,
    /// Out of service (a damage aboard).
    pub down: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerView {
    pub name: String,
    /// Points the ship has now, to split between channels.
    pub points: u32,
    pub channels: Vec<ChannelView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelView {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub max: u32,
    pub note: String,
}

/// The caller at their station.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeView {
    pub id: String,
    pub station: Option<String>,
    pub actions: u32,
    pub attacks: u32,
    pub done: bool,
    /// The crew's turn, and I still act.
    pub my_turn: bool,
    /// What changing station costs, in actions.
    pub station_cost: u32,
    pub options: Vec<ActionView>,
}

/// What an action needs to be aimed.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AimKind {
    None,
    Ship,
    Cell,
    Power,
    Damage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: u32,
    pub attack: bool,
    pub check: Option<i32>,
    pub aim: AimKind,
    /// Enough actions (and an attack) left for it now.
    pub usable: bool,
    /// Ships it can aim at now, with the weapon for a shot.
    pub targets: Vec<TargetView>,
    /// Cells a manoeuvre can end on, and how far the bow may turn.
    pub reach: Vec<Cell>,
    pub turns: u32,
    /// Damages aboard it can fix: their index in the ship's list.
    pub damages: Vec<DamageView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetView {
    pub ship: String,
    pub weapon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageView {
    pub index: usize,
    pub name: String,
}

fn known(s: &Ship) -> bool {
    s.side == Side::Party || s.scanned
}

fn ship_view(s: &Ship) -> ShipView {
    let k = known(s);
    let show = |x: i32| k.then_some(x);
    ShipView {
        id: s.id.clone(),
        name: s.name.clone(),
        party: s.side == Side::Party,
        at: s.at,
        facing: s.facing,
        standing: s.standing,
        known: k,
        hull: show(s.hull),
        max_hull: show(s.max_hull),
        screen: show(s.screen),
        max_screen: show(s.max_screen),
        armor: show(s.armor),
        morale: s.morale.filter(|_| k),
        max_morale: s.max_morale.filter(|_| k),
        damages: if k {
            s.damages.iter().map(|d| d.name.clone()).collect()
        } else {
            Vec::new()
        },
    }
}

/// An event as a player may read it: an unscanned enemy's gauges out,
/// and the damage it rolled (a hull threshold crossed) left out, as its
/// damages are in [`ship_view`].
fn event_for_players(b: &Battle, e: &BattleEvent) -> Option<BattleEvent> {
    let hidden = |id: &str| b.ship(id).is_some_and(|s| !known(s));
    let mut e = e.clone();
    match &mut e {
        BattleEvent::DamageRolled { ship, .. } if hidden(ship) => return None,
        BattleEvent::Fired {
            target,
            screen_after,
            hull_after,
            ..
        } if hidden(target) => {
            *screen_after = None;
            *hull_after = None;
        }
        BattleEvent::MoraleRoll {
            target,
            morale_after,
            ..
        } if hidden(target) => *morale_after = None,
        _ => {}
    }
    Some(e)
}

/// What crew member `me` can do at their station now, each action with
/// what it can aim at.
#[must_use]
pub fn options(rules: &RuleSystem, b: &Battle, me: &str) -> Vec<ActionView> {
    let (Some(v), Some(member)) = (rules.vehicles.as_ref(), b.crew_member(me)) else {
        return Vec::new();
    };
    let Some(station) = member.station.as_deref().and_then(|s| v.station(s)) else {
        return Vec::new();
    };
    let party = b.party_ship();
    let enemies: Vec<&Ship> = b
        .ships
        .iter()
        .filter(|s| s.side == Side::Opposition && s.afloat())
        .collect();
    let down = b.stations_down().contains(&station.id);
    station
        .actions
        .iter()
        .map(|a| {
            let mut targets = Vec::new();
            let mut reach = Vec::new();
            let mut turns = 0;
            let mut damages = Vec::new();
            let aim = match &a.effect {
                VehicleEffect::Fire { .. } => {
                    for t in &enemies {
                        for w in b.weapons_on(v, party, t) {
                            if w.station.as_deref() == Some(station.id.as_str()) {
                                targets.push(TargetView {
                                    ship: t.id.clone(),
                                    weapon: Some(w.id.clone()),
                                });
                            }
                        }
                    }
                    AimKind::Ship
                }
                VehicleEffect::Lock { .. } | VehicleEffect::Scan | VehicleEffect::Jam { .. } => {
                    targets.extend(enemies.iter().map(|t| TargetView {
                        ship: t.id.clone(),
                        weapon: None,
                    }));
                    AimKind::Ship
                }
                VehicleEffect::BreakMorale { .. } => {
                    targets.extend(enemies.iter().filter(|t| t.morale.is_some()).map(|t| {
                        TargetView {
                            ship: t.id.clone(),
                            weapon: None,
                        }
                    }));
                    AimKind::Ship
                }
                VehicleEffect::Maneuver {
                    multiplier,
                    turns: t,
                } => {
                    reach = b.reachable(v, &party.id, *multiplier).into_keys().collect();
                    reach.push(party.at);
                    turns = *t;
                    AimKind::Cell
                }
                VehicleEffect::Reroute => AimKind::Power,
                VehicleEffect::Fix { damages: fixes } => {
                    damages = party
                        .damages
                        .iter()
                        .enumerate()
                        .filter(|(_, d)| fixes.contains(&d.id))
                        .map(|(index, d)| DamageView {
                            index,
                            name: d.name.clone(),
                        })
                        .collect();
                    AimKind::Damage
                }
                _ => AimKind::None,
            };
            ActionView {
                id: a.id.clone(),
                name: a.name.clone(),
                description: a.description.clone(),
                cost: a.cost,
                attack: a.attack,
                check: a.check,
                aim,
                usable: !down
                    && member.actions >= a.cost
                    && (!a.attack || member.attacks > 0)
                    && (!a.attack || !a.cost.gt(&1) || b.heavy_allowed(v)),
                targets,
                reach,
                turns,
                damages,
            }
        })
        .collect()
}

/// The battle as the player whose crew id is `me` (none: a spectator)
/// may see it.
#[must_use]
pub fn project_battle(
    stored: &StoredBattle,
    events: &[BattleEvent],
    rules: Option<&RuleSystem>,
    me: Option<&str>,
) -> BattleView {
    let b = &stored.battle;
    let v = rules.and_then(|r| r.vehicles.as_ref());
    let crew_turn = b.crew_turn() && b.boarding.is_none();
    let holder = |station: &str| {
        b.crew
            .iter()
            .find(|c| c.station.as_deref() == Some(station))
            .map(|c| c.name.clone())
    };
    let down = b.stations_down();
    let party = b.party_ship();
    let me_view = me.and_then(|id| b.crew_member(id)).map(|m| MeView {
        id: m.id.clone(),
        station: m.station.clone(),
        actions: m.actions,
        attacks: m.attacks,
        done: m.done,
        my_turn: crew_turn && !b.is_over() && !m.done,
        station_cost: v.map_or(0, |v| v.station_change_cost),
        options: rules.map_or_else(Vec::new, |r| options(r, b, &m.id)),
    });
    let won = b.end.as_ref().map(|e| e.winner == Some(Side::Party));
    BattleView {
        live: !b.is_over(),
        boarding: b.boarding.is_some(),
        round: b.round,
        crew_turn,
        active: b.active().map(|u| u.name.clone()),
        map: b.map.project(Viewer::Player),
        current: b.current,
        gauges: Gauges {
            hull: v.map_or_else(String::new, |v| v.hull.name.clone()),
            screen: v.and_then(|v| v.screen.as_ref().map(|s| s.name.clone())),
            armor: v.map_or_else(String::new, |v| v.armor.name.clone()),
            morale: v.map_or_else(String::new, |v| v.morale.name.clone()),
            power: v.and_then(|v| v.power.as_ref().map(|p| p.name.clone())),
        },
        ships: b.ships.iter().map(ship_view).collect(),
        crew: b
            .crew
            .iter()
            .map(|c| CrewView {
                id: c.id.clone(),
                name: c.name.clone(),
                station: c.station.clone(),
                npc: c.npc,
                done: c.done,
                mine: Some(c.id.as_str()) == me,
            })
            .collect(),
        stations: v.map_or_else(Vec::new, |v| {
            v.stations
                .iter()
                .map(|s| StationView {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    description: s.description.clone(),
                    holder: holder(&s.id),
                    down: down.contains(&s.id),
                })
                .collect()
        }),
        power: v.and_then(|v| {
            let p = v.power.as_ref()?;
            Some(PowerView {
                name: p.name.clone(),
                points: b.power_points(v, party),
                channels: p
                    .channels
                    .iter()
                    .map(|c| {
                        let level = party.power.get(&c.id).copied().unwrap_or(0);
                        ChannelView {
                            id: c.id.clone(),
                            name: c.name.clone(),
                            level,
                            max: u32::try_from(c.levels.len().saturating_sub(1)).unwrap_or(0),
                            note: c
                                .levels
                                .get(level as usize)
                                .map(|l| l.note.clone())
                                .unwrap_or_default(),
                        }
                    })
                    .collect(),
            })
        }),
        me: me_view,
        events: events
            .iter()
            .filter_map(|e| event_for_players(b, e))
            .collect(),
        won,
        reason: b.end.as_ref().map(|e| e.reason),
    }
}

#[cfg(test)]
mod tests {
    use promptus_shared::rules::check::RollBreakdown;
    use promptus_shared::story::RuleSystemRef;
    use promptus_shared::vehicle::scenario::VehicleScenario;
    use uuid::Uuid;

    use super::*;

    /// The Greyhound's interception at its start, as the server stores it.
    fn interception() -> (StoredBattle, &'static RuleSystem) {
        let rules = crate::content::preset(&RuleSystemRef {
            id: "corsaires".into(),
            version: 1,
        })
        .expect("the Corsaires preset");
        let scenario = VehicleScenario::from_yaml(include_str!(
            "../../../../content/scenarios/corsaires/vaisseau/interception-greyhound.yaml"
        ))
        .unwrap();
        let map = crate::content::world_map("corsaires", &scenario.map).expect("the sea map");
        let battle = scenario.battle(rules, map, 7).unwrap();
        let stored = StoredBattle {
            id: Uuid::nil(),
            session_id: None,
            node: "sc_interception_greyhound".into(),
            status: "live".into(),
            version: 1,
            battle,
            proposal: None,
            boarding_encounter: None,
            started_at: chrono::Utc::now(),
            ended_at: None,
        };
        (stored, rules)
    }

    fn shot(roll: RollBreakdown) -> BattleEvent {
        BattleEvent::Fired {
            ship: "la_machoire".into(),
            weapon: "mousquets".into(),
            by: Some("bretteur".into()),
            target: "greyhound".into(),
            roll,
            hit: true,
            damage: 4,
            screen_after: Some(3),
            hull_after: Some(17),
        }
    }

    #[test]
    fn an_unscanned_enemy_keeps_its_numbers_until_the_crew_reads_it() {
        let (mut stored, rules) = interception();
        let roll: RollBreakdown = serde_json::from_value(serde_json::json!({
            "die": "d20", "faces": [12], "natural": 12, "advantage": "normal",
            "modifiers": [], "total": 12, "target": null, "band": null,
        }))
        .unwrap();
        // The shot crossed a hull threshold: a damage rolled on board.
        let rolled = BattleEvent::DamageRolled {
            ship: "greyhound".into(),
            face: 3,
            damage: "incendie".into(),
            name: "GMONLY-damage".into(),
            station: None,
            crew: None,
        };
        let events = [shot(roll), rolled];
        let view = project_battle(&stored, &events, Some(rules), Some("bretteur"));
        let grey = view.ships.iter().find(|s| s.id == "greyhound").unwrap();
        assert!(!grey.known && grey.hull.is_none() && grey.morale.is_none());
        let json = serde_json::to_value(&view.events).unwrap();
        assert!(json[0]["hull_after"].is_null(), "{json}");
        assert!(json[0]["screen_after"].is_null(), "{json}");
        assert!(!json.to_string().contains("GMONLY"), "{json}");
        // The party ship is always known; my station's actions come with
        // what they can aim at.
        let me = view.me.as_ref().unwrap();
        assert_eq!(me.station.as_deref(), Some("pont"));
        assert!(!me.options.is_empty());

        // Scanned: the crew sees it all.
        let i = stored
            .battle
            .ships
            .iter()
            .position(|s| s.id == "greyhound")
            .unwrap();
        stored.battle.ships[i].scanned = true;
        let view = project_battle(&stored, &events, Some(rules), Some("bretteur"));
        let grey = view.ships.iter().find(|s| s.id == "greyhound").unwrap();
        assert!(grey.known && grey.hull.is_some());
        let json = serde_json::to_value(&view.events).unwrap();
        assert_eq!(json[0]["hull_after"], 17);
        assert_eq!(json[1]["name"], "GMONLY-damage");
    }
}
