//! The fight simulator: a [`Scenario`] played N times with seeded dice
//! under one version of the rules, and what came out of it — who won,
//! how long it took in rounds and in minutes at the table, who dealt and
//! took the damage, who fell, who ran, the XP each class earned. Two
//! versions of the rules played on the same seeds give a [`Delta`]: the
//! effect of a rule change, before a table ever sees it.
//!
//! Pure and deterministic: the same scenario, rules, policies and seed
//! give the same report, field for field.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::maps::Map;
use crate::rules::RuleSystem;
use crate::rules::events::{Event, RollPurpose};
use crate::rules::sheet::{Origin, Side};

use super::fight::{EndReason, FightEvent, Standing};
use super::run::{FightLog, Limits, LogEntry, run_fight};
use super::scenario::{PolicyKind, Scenario, ScenarioError};

/// How long a fight lasts at a real, remote table. INTERPRETATION, to be
/// set against a timed fight (none of Romain's games was timed):
///
/// - **2 minutes of setup**: tokens placed, twelve initiatives read out,
///   the first turn explained (the dock fight is a tutorial);
/// - **60 s per character turn**: with two actions, a remote player
///   picks a card, rolls, reads the result; the GM narrates it;
/// - **30 s per adversary turn**: the GM picks and rolls for one stat
///   block, faster than a player but not free;
/// - **5 s per empty turn**: someone knocked out or with nothing to do,
///   the GM just calls the next name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TimeModel {
    pub setup_seconds: u32,
    pub party_turn_seconds: u32,
    pub opposition_turn_seconds: u32,
    pub idle_turn_seconds: u32,
}

impl Default for TimeModel {
    fn default() -> Self {
        Self {
            setup_seconds: 120,
            party_turn_seconds: 60,
            opposition_turn_seconds: 30,
            idle_turn_seconds: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SimParams {
    pub fights: u32,
    /// Fight `i` (from 0) is played with seed `seed + i`.
    pub seed: u64,
    pub time: TimeModel,
    pub limits: Limits,
}

impl SimParams {
    pub fn new(fights: u32, seed: u64) -> Self {
        Self {
            fights,
            seed,
            time: TimeModel::default(),
            limits: Limits::default(),
        }
    }
}

/// Counts for one group of fighters (a class, or one stat block shared
/// by several adversaries), summed over every fight.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct GroupTally {
    /// Hit points actually removed (no overkill below 0).
    pub damage_dealt: u64,
    pub damage_taken: u64,
    pub attacks: u64,
    /// Attack rolls that succeeded (criticals included).
    pub hits: u64,
    pub knocked_out: u64,
    pub out_of_scene: u64,
    pub fled: u64,
    pub defeated: u64,
    pub xp: u64,
}

impl GroupTally {
    fn add(&mut self, o: &Self) {
        self.damage_dealt += o.damage_dealt;
        self.damage_taken += o.damage_taken;
        self.attacks += o.attacks;
        self.hits += o.hits;
        self.knocked_out += o.knocked_out;
        self.out_of_scene += o.out_of_scene;
        self.fled += o.fled;
        self.defeated += o.defeated;
        self.xp += o.xp;
    }

    /// Hits over attacks, 0 when it never attacked.
    pub fn hit_rate(&self) -> f64 {
        ratio(self.hits, self.attacks)
    }
}

/// What one fight gave.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FightTally {
    pub winner: Option<Side>,
    pub reason: EndReason,
    pub rounds: u32,
    pub party_turns: u32,
    pub opposition_turns: u32,
    pub idle_turns: u32,
    pub seconds: u32,
    pub refusals: u32,
    /// Damage with no actor: conditions ticking at the end of a turn.
    pub damage_from_conditions: u64,
    /// By group id (class or stat block).
    pub groups: BTreeMap<String, GroupTally>,
}

/// The group a combatant counts in: its class or its stat block.
fn group_of(log: &FightLog, id: &str) -> Option<String> {
    log.fight.combatant(id).map(|c| match &c.origin {
        Origin::Class(class) => class.clone(),
        Origin::Adversary(block) => block.clone(),
    })
}

/// Reads one fight's log into counts.
pub fn tally(log: &FightLog, time: &TimeModel) -> FightTally {
    let mut groups: BTreeMap<String, GroupTally> = BTreeMap::new();
    for c in log.fight.scene.combatants.values() {
        groups
            .entry(group_of(log, &c.id).unwrap_or_default())
            .or_default();
    }
    fn slot<'g>(
        groups: &'g mut BTreeMap<String, GroupTally>,
        log: &FightLog,
        id: &str,
    ) -> &'g mut GroupTally {
        groups
            .entry(group_of(log, id).unwrap_or_else(|| id.to_string()))
            .or_default()
    }
    let mut from_conditions = 0;
    let (mut party_turns, mut opposition_turns, mut idle_turns) = (0, 0, 0);
    // The turn under way: who, and whether they did anything.
    let mut turn: Option<(String, bool)> = None;
    let mut close = |turn: &mut Option<(String, bool)>| {
        if let Some((who, active)) = turn.take() {
            let side = log.fight.combatant(&who).map(|c| c.side);
            match (active, side) {
                (false, _) => idle_turns += 1,
                (true, Some(Side::Party)) => party_turns += 1,
                (true, _) => opposition_turns += 1,
            }
        }
    };
    for entry in &log.log {
        let LogEntry::Events { events } = entry else {
            continue;
        };
        let mut actor: Option<&str> = None;
        for e in events {
            match e {
                FightEvent::TurnStarted { who, .. } => {
                    close(&mut turn);
                    turn = Some((who.clone(), false));
                }
                FightEvent::TurnEnded { .. } | FightEvent::Ended { .. } => close(&mut turn),
                FightEvent::Moved { who, .. }
                | FightEvent::FleeRoll { who, .. }
                | FightEvent::FleeFailed { who }
                | FightEvent::Fled { who } => {
                    if let Some((current, active)) = turn.as_mut()
                        && current == who
                    {
                        *active = true;
                    }
                }
                FightEvent::Acted { who, .. } => {
                    actor = Some(who);
                    if let Some((current, active)) = turn.as_mut()
                        && current == who
                    {
                        *active = true;
                    }
                }
                FightEvent::Rules { event } => match event {
                    Event::Damaged {
                        target,
                        hp_before,
                        hp_after,
                        ..
                    } => {
                        let amount = u64::try_from((hp_before - hp_after).max(0)).unwrap_or(0);
                        slot(&mut groups, log, target).damage_taken += amount;
                        match actor {
                            Some(a) => slot(&mut groups, log, a).damage_dealt += amount,
                            None => from_conditions += amount,
                        }
                    }
                    Event::Roll {
                        roller,
                        purpose: RollPurpose::Attack,
                        breakdown,
                        ..
                    } => {
                        let g = slot(&mut groups, log, roller);
                        g.attacks += 1;
                        if breakdown.band.is_some_and(|b| b.is_success()) {
                            g.hits += 1;
                        }
                    }
                    Event::KnockedOut { target } => slot(&mut groups, log, target).knocked_out += 1,
                    _ => {}
                },
                _ => {}
            }
        }
    }
    close(&mut turn);
    for (id, standing) in &log.fight.standing {
        let g = slot(&mut groups, log, id);
        match standing {
            Standing::InFight => {}
            Standing::Defeated => g.defeated += 1,
            Standing::OutOfScene => g.out_of_scene += 1,
            Standing::Fled => g.fled += 1,
        }
    }
    let end = log.fight.end.as_ref();
    for (pc, xp) in end.map(|e| &e.xp).into_iter().flatten() {
        slot(&mut groups, log, pc).xp += u64::from(*xp);
    }
    let seconds = time.setup_seconds
        + party_turns * time.party_turn_seconds
        + opposition_turns * time.opposition_turn_seconds
        + idle_turns * time.idle_turn_seconds;
    FightTally {
        winner: end.and_then(|e| e.winner),
        reason: end.map_or(EndReason::StoppedByGm, |e| e.reason),
        rounds: end.map_or(log.fight.round, |e| e.rounds),
        party_turns,
        opposition_turns,
        idle_turns,
        seconds,
        refusals: u32::try_from(log.refusals().count()).unwrap_or(u32::MAX),
        damage_from_conditions: from_conditions,
        groups,
    }
}

/// One fight of the scenario, played to its end.
pub fn play(
    system: &RuleSystem,
    scenario: &Scenario,
    map: &Map,
    party: PolicyKind,
    opposition: PolicyKind,
    seed: u64,
    limits: Limits,
) -> Result<FightLog, ScenarioError> {
    let (step, mut dice) = scenario.start(system, map, seed)?;
    let mut party = scenario.policy(Side::Party, party);
    let mut opposition = scenario.policy(Side::Opposition, opposition);
    Ok(run_fight(
        system,
        step,
        party.as_mut(),
        opposition.as_mut(),
        &mut dice,
        limits,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Spread {
    pub mean: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
}

impl Spread {
    pub(crate) fn of(mut values: Vec<f64>) -> Self {
        if values.is_empty() {
            return Self {
                mean: 0.0,
                median: 0.0,
                min: 0.0,
                max: 0.0,
            };
        }
        values.sort_by(f64::total_cmp);
        let n = values.len();
        let median = if n % 2 == 1 {
            values[n / 2]
        } else {
            f64::midpoint(values[n / 2 - 1], values[n / 2])
        };
        Self {
            mean: values.iter().sum::<f64>() / n as f64,
            median,
            min: values[0],
            max: values[n - 1],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Outcomes {
    pub party_wins: u32,
    pub opposition_wins: u32,
    /// Both sides down at once.
    pub draws: u32,
    /// Stopped after `Limits::max_rounds`, or stalled.
    pub stopped: u32,
}

/// Per-fight means of a [`GroupTally`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct GroupMeans {
    pub damage_dealt: f64,
    pub damage_taken: f64,
    pub hit_rate: f64,
    pub knocked_out: f64,
    pub out_of_scene: f64,
    pub fled: f64,
    pub defeated: f64,
    pub xp: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroupStats {
    /// Class or stat block id.
    pub id: String,
    pub name: String,
    pub side: Side,
    /// Fighters of this group in the scenario.
    pub members: u32,
    /// Summed over every fight.
    pub totals: GroupTally,
    pub per_fight: GroupMeans,
}

/// N fights of one scenario under one version of the rules.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimReport {
    pub scenario: String,
    pub rules: String,
    pub version: u32,
    pub party_policy: PolicyKind,
    pub opposition_policy: PolicyKind,
    pub params: SimParams,
    pub outcomes: Outcomes,
    pub party_win_rate: f64,
    pub rounds: Spread,
    /// Estimated minutes at the table, per [`TimeModel`].
    pub minutes: Spread,
    /// Mean turns per fight: characters', adversaries', empty ones.
    pub party_turns: f64,
    pub opposition_turns: f64,
    pub idle_turns: f64,
    /// Commands the engine refused; the policies only ask for what it
    /// accepts, so anything but 0 is a bug.
    pub refusals: u32,
    pub damage_from_conditions: u64,
    /// The party's classes, then the stat blocks, in scenario order.
    pub groups: Vec<GroupStats>,
}

fn ratio(a: u64, b: u64) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}

/// Plays the scenario `params.fights` times and sums it up.
pub fn simulate(
    system: &RuleSystem,
    scenario: &Scenario,
    map: &Map,
    party: PolicyKind,
    opposition: PolicyKind,
    params: &SimParams,
) -> Result<SimReport, ScenarioError> {
    let mut tallies = Vec::new();
    for i in 0..params.fights {
        let log = play(
            system,
            scenario,
            map,
            party,
            opposition,
            params.seed.wrapping_add(u64::from(i)),
            params.limits,
        )?;
        tallies.push(tally(&log, &params.time));
    }
    Ok(aggregate(
        system, scenario, party, opposition, params, &tallies,
    ))
}

fn aggregate(
    system: &RuleSystem,
    scenario: &Scenario,
    party: PolicyKind,
    opposition: PolicyKind,
    params: &SimParams,
    tallies: &[FightTally],
) -> SimReport {
    let n = tallies.len().max(1) as f64;
    let mut outcomes = Outcomes::default();
    for t in tallies {
        match (t.reason, t.winner) {
            (EndReason::SideDown, Some(Side::Party)) => outcomes.party_wins += 1,
            (EndReason::SideDown, Some(Side::Opposition)) => outcomes.opposition_wins += 1,
            (EndReason::SideDown, None) => outcomes.draws += 1,
            _ => outcomes.stopped += 1,
        }
    }
    let mut totals: BTreeMap<&str, GroupTally> = BTreeMap::new();
    for t in tallies {
        for (id, g) in &t.groups {
            totals.entry(id).or_default().add(g);
        }
    }
    let mut groups: Vec<GroupStats> = Vec::new();
    for (side, spec) in [
        (Side::Party, &scenario.party),
        (Side::Opposition, &scenario.opposition),
    ] {
        for f in &spec.fighters {
            let id = f.class.as_ref().or(f.adversary.as_ref()).expect("loaded");
            if let Some(g) = groups.iter_mut().find(|g| &g.id == id) {
                g.members += 1;
                continue;
            }
            let name = match side {
                Side::Party => system.class(id).map(|c| c.name.clone()),
                Side::Opposition => system.adversary(id).map(|a| a.name.clone()),
            }
            .unwrap_or_else(|| id.clone());
            let t = totals.get(id.as_str()).copied().unwrap_or_default();
            groups.push(GroupStats {
                id: id.clone(),
                name,
                side,
                members: 1,
                totals: t,
                per_fight: GroupMeans {
                    damage_dealt: t.damage_dealt as f64 / n,
                    damage_taken: t.damage_taken as f64 / n,
                    hit_rate: t.hit_rate(),
                    knocked_out: t.knocked_out as f64 / n,
                    out_of_scene: t.out_of_scene as f64 / n,
                    fled: t.fled as f64 / n,
                    defeated: t.defeated as f64 / n,
                    xp: t.xp as f64 / n,
                },
            });
        }
    }
    let mean = |f: fn(&FightTally) -> u32| tallies.iter().map(|t| f64::from(f(t))).sum::<f64>() / n;
    SimReport {
        scenario: scenario.id.clone(),
        rules: system.id.clone(),
        version: system.version,
        party_policy: party,
        opposition_policy: opposition,
        params: *params,
        outcomes,
        party_win_rate: f64::from(outcomes.party_wins) / n,
        rounds: Spread::of(tallies.iter().map(|t| f64::from(t.rounds)).collect()),
        minutes: Spread::of(
            tallies
                .iter()
                .map(|t| f64::from(t.seconds) / 60.0)
                .collect(),
        ),
        party_turns: mean(|t| t.party_turns),
        opposition_turns: mean(|t| t.opposition_turns),
        idle_turns: mean(|t| t.idle_turns),
        refusals: tallies.iter().map(|t| t.refusals).sum(),
        damage_from_conditions: tallies.iter().map(|t| t.damage_from_conditions).sum(),
        groups,
    }
}

/// How one group changed from a base run to a variant run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroupDelta {
    pub id: String,
    pub name: String,
    pub side: Side,
    pub damage_dealt: f64,
    pub damage_taken: f64,
    pub hit_rate: f64,
    pub xp: f64,
}

/// A variant's run minus the base run, per fight.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Delta {
    pub party_win_rate: f64,
    pub rounds: f64,
    pub minutes: f64,
    pub groups: Vec<GroupDelta>,
}

/// The effect of a rule change: `other` minus `base`, both played on
/// the same scenario, policies and seeds.
pub fn compare(base: &SimReport, other: &SimReport) -> Delta {
    let groups = base
        .groups
        .iter()
        .filter_map(|b| {
            let o = other.groups.iter().find(|o| o.id == b.id)?;
            Some(GroupDelta {
                id: b.id.clone(),
                name: b.name.clone(),
                side: b.side,
                damage_dealt: o.per_fight.damage_dealt - b.per_fight.damage_dealt,
                damage_taken: o.per_fight.damage_taken - b.per_fight.damage_taken,
                hit_rate: o.per_fight.hit_rate - b.per_fight.hit_rate,
                xp: o.per_fight.xp - b.per_fight.xp,
            })
        })
        .collect();
    Delta {
        party_win_rate: other.party_win_rate - base.party_win_rate,
        rounds: other.rounds.mean - base.rounds.mean,
        minutes: other.minutes.mean - base.minutes.mean,
        groups,
    }
}
