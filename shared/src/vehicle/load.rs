//! Validation of the `vehicles:` block, run by the rule system's loader:
//! it refuses what the battle engine cannot run (a station rolling an
//! ability that does not exist, a weapon on no band, a channel starting
//! above its levels…), with the loader's codes and paths.

use std::collections::BTreeSet;

use crate::rules::load::{ErrorCode, RuleError};
use crate::rules::model::RuleSystem;

use super::model::{VehicleEffect, VehicleRules};

struct V<'a> {
    s: &'a RuleSystem,
    errors: Vec<RuleError>,
}

impl V<'_> {
    fn err(&mut self, code: ErrorCode, path: impl Into<String>, detail: impl Into<String>) {
        self.errors.push(RuleError {
            code,
            path: path.into(),
            detail: detail.into(),
        });
    }

    fn ability(&mut self, id: &str, path: String) {
        if !self.s.abilities.iter().any(|a| a.id == id) {
            self.err(
                ErrorCode::UnknownAbility,
                path,
                format!("no ability `{id}`"),
            );
        }
    }

    fn unique<'i>(&mut self, ids: impl IntoIterator<Item = &'i str>, path: &str) {
        let mut seen = BTreeSet::new();
        for id in ids {
            if id.trim().is_empty() {
                self.err(ErrorCode::EmptyField, path, "an id is empty");
            } else if !seen.insert(id) {
                self.err(
                    ErrorCode::DuplicateId,
                    path,
                    format!("`{id}` appears twice"),
                );
            }
        }
    }

    fn positive(&mut self, n: i32, path: String) {
        if n < 1 {
            self.err(ErrorCode::InvalidValue, path, "at least 1");
        }
    }
}

/// Errors of `s.vehicles`, if any.
pub fn validate(s: &RuleSystem) -> Vec<RuleError> {
    let Some(v) = &s.vehicles else {
        return Vec::new();
    };
    let mut c = V {
        s,
        errors: Vec::new(),
    };
    check(&mut c, v);
    c.errors
}

fn check(c: &mut V<'_>, v: &VehicleRules) {
    let p = "vehicles";
    if c.s.turn_context(&v.context).is_none() {
        c.err(
            ErrorCode::InvalidValue,
            format!("{p}.context"),
            format!("no turn context `{}`", v.context),
        );
    }
    if c.s.action_kind(&v.attack_kind).is_none() {
        c.err(
            ErrorCode::UnknownActionKind,
            format!("{p}.attack_kind"),
            format!("no action kind `{}`", v.attack_kind),
        );
    }
    c.unique(
        v.ranges.iter().map(|r| r.id.as_str()),
        &format!("{p}.ranges"),
    );
    if v.ranges.is_empty() {
        c.err(
            ErrorCode::EmptyField,
            format!("{p}.ranges"),
            "at least one band",
        );
    }
    if v.ranges.windows(2).any(|w| w[1].max <= w[0].max) {
        c.err(
            ErrorCode::InvalidValue,
            format!("{p}.ranges"),
            "bands go from nearest to farthest",
        );
    }
    if v.station(&v.initiative_station).is_none() {
        c.err(
            ErrorCode::InvalidValue,
            format!("{p}.initiative_station"),
            format!("no station `{}`", v.initiative_station),
        );
    }
    if let Some(power) = &v.power {
        c.unique(
            power.channels.iter().map(|ch| ch.id.as_str()),
            &format!("{p}.power.channels"),
        );
        let starts: u32 = power.channels.iter().map(|ch| ch.start).sum();
        if starts > power.points {
            c.err(
                ErrorCode::InvalidValue,
                format!("{p}.power"),
                format!(
                    "channels start with {starts} points out of {}",
                    power.points
                ),
            );
        }
        for ch in &power.channels {
            if ch.levels.is_empty() || ch.start as usize >= ch.levels.len() {
                c.err(
                    ErrorCode::InvalidValue,
                    format!("{p}.power.channels[{}]", ch.id),
                    "a channel starts on one of its levels",
                );
            }
        }
    }
    let damage_ids: Vec<&str> = v
        .damage
        .iter()
        .flat_map(|t| t.entries.iter().map(|e| e.id.as_str()))
        .collect();
    if let Some(table) = &v.damage {
        c.unique(damage_ids.iter().copied(), &format!("{p}.damage.entries"));
        for e in &table.entries {
            if e.from > e.to || e.to > table.dice.count * table.dice.faces {
                c.err(
                    ErrorCode::InvalidValue,
                    format!("{p}.damage.entries[{}]", e.id),
                    format!("faces {}..={} do not fit {}", e.from, e.to, table.dice),
                );
            }
            if let Some(f) = &e.fix {
                c.ability(
                    &f.ability,
                    format!("{p}.damage.entries[{}].fix.ability", e.id),
                );
            }
        }
    }
    c.unique(
        v.stations.iter().map(|s| s.id.as_str()),
        &format!("{p}.stations"),
    );
    c.unique(
        v.stations
            .iter()
            .flat_map(|s| s.actions.iter().map(|a| a.id.as_str())),
        &format!("{p}.stations.actions"),
    );
    for st in &v.stations {
        let sp = format!("{p}.stations[{}]", st.id);
        c.ability(&st.ability, format!("{sp}.ability"));
        for class in &st.suits {
            if c.s.class(class).is_none() {
                c.err(
                    ErrorCode::InvalidValue,
                    format!("{sp}.suits"),
                    format!("no class `{class}`"),
                );
            }
        }
        for a in &st.actions {
            let ap = format!("{sp}.actions[{}]", a.id);
            if a.cost == 0 {
                c.err(ErrorCode::InvalidValue, format!("{ap}.cost"), "at least 1");
            }
            match &a.effect {
                VehicleEffect::Fix { damages } => {
                    for d in damages {
                        if !damage_ids.contains(&d.as_str()) {
                            c.err(
                                ErrorCode::InvalidValue,
                                format!("{ap}.effect.fix"),
                                format!("no damage `{d}` in the damage table"),
                            );
                        }
                    }
                }
                VehicleEffect::Reroute if v.power.is_none() => c.err(
                    ErrorCode::InvalidValue,
                    format!("{ap}.effect"),
                    "rerouting needs a `power` block",
                ),
                VehicleEffect::Recharge { .. } if v.screen.is_none() => c.err(
                    ErrorCode::InvalidValue,
                    format!("{ap}.effect"),
                    "recharging needs a `screen` gauge",
                ),
                VehicleEffect::Maneuver { multiplier, .. } if *multiplier == 0 => c.err(
                    ErrorCode::InvalidValue,
                    format!("{ap}.effect"),
                    "a manoeuvre multiplies the speed by at least 1",
                ),
                _ => {}
            }
        }
    }
    c.unique(v.ships.iter().map(|s| s.id.as_str()), &format!("{p}.ships"));
    for ship in &v.ships {
        let sp = format!("{p}.ships[{}]", ship.id);
        c.positive(ship.hull, format!("{sp}.hull"));
        if ship.screen > 0 && v.screen.is_none() {
            c.err(
                ErrorCode::InvalidValue,
                format!("{sp}.screen"),
                "no `screen` gauge in the vehicle rules",
            );
        }
        if let Some(m) = ship.morale {
            c.positive(m, format!("{sp}.morale"));
        }
        c.unique(
            ship.weapons.iter().map(|w| w.id.as_str()),
            &format!("{sp}.weapons"),
        );
        for w in &ship.weapons {
            let wp = format!("{sp}.weapons[{}]", w.id);
            if v.band(&w.range).is_none() {
                c.err(
                    ErrorCode::InvalidValue,
                    format!("{wp}.range"),
                    format!("no range band `{}`", w.range),
                );
            }
            if w.arcs.is_empty() {
                c.err(
                    ErrorCode::EmptyField,
                    format!("{wp}.arcs"),
                    "at least one arc",
                );
            }
            if let Some(st) = &w.station
                && v.station(st).is_none()
            {
                c.err(
                    ErrorCode::InvalidValue,
                    format!("{wp}.station"),
                    format!("no station `{st}`"),
                );
            }
        }
    }
}
