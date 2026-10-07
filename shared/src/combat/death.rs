//! Death saves (`engine/save-against-death`, planche « Mourir »), when
//! the rule system plays them (`zero_hp: { rule: death_saves }`).
//!
//! A character brought to 0 hit points falls under the system's
//! condition and starts dying. On each of their turns they roll the
//! check die against the rule's difficulty: a success or a failure
//! marks a box; a natural 1 marks two failures; a natural 20 brings them
//! back at 1 hit point. A hit while down marks a failure, two on a
//! critical. Enough successes and they are stable — down, no longer
//! rolling. A heal lifts them and clears the boxes. Enough failures and
//! the engine **proposes** the death: nothing more happens until the GM
//! confirms it or decides another outcome (rule 1, the GM has the last
//! word). Adversaries never roll: at 0 they are defeated.
//!
//! INTERPRETATION: a natural 20 ends the turn like any other save — the
//! character is up for their next one. When the fight ends, those still
//! dying are stabilised if the party won, else their death is proposed.

use serde::{Deserialize, Serialize};

use crate::rules::RuleSystem;
use crate::rules::action::Refusal;
use crate::rules::check::OutcomeBand;
use crate::rules::conditions;
use crate::rules::dice::DiceSource;
use crate::rules::events::{Event, RollPurpose};
use crate::rules::model::ZeroHpRule;

use super::fight::{CombatRefusal, Fight, FightEvent, Standing, Step};

/// The boxes of a dying character.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeathTrack {
    pub successes: u32,
    pub failures: u32,
    /// Enough successes (or the GM spared them): no more saves.
    pub stable: bool,
    /// Enough failures: the death waits for the GM's word.
    pub proposed: bool,
}

/// The GM's word on a proposed death.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathCall {
    /// The character dies.
    Die,
    /// Another outcome: down and stable.
    Spare,
}

/// `(difficulty, successes, failures)` of the system's death saves.
fn rule(system: &RuleSystem) -> Option<(i32, u32, u32)> {
    match &system.zero_hp {
        ZeroHpRule::DeathSaves {
            difficulty,
            successes,
            failures,
            ..
        } => Some((*difficulty, *successes, *failures)),
        ZeroHpRule::KnockedOut { .. } => None,
    }
}

impl Fight {
    /// Whether `who` is a character the system's death saves apply to,
    /// down at 0 hit points.
    fn dying_character(&self, system: &RuleSystem, who: &str) -> bool {
        rule(system).is_some()
            && self
                .combatant(who)
                .is_some_and(|c| c.progress.is_some() && c.hit_points == 0)
    }

    /// `who`'s turn waits for a death save: dying, not stable, no death
    /// proposed.
    #[must_use]
    pub fn save_due(&self, system: &RuleSystem, who: &str) -> bool {
        self.in_fight(who)
            && self.dying_character(system, who)
            && self.dying.get(who).is_none_or(|t| !t.stable && !t.proposed)
    }

    /// The deaths waiting for the GM's word.
    pub fn proposed_deaths(&self) -> impl Iterator<Item = &str> {
        self.dying
            .iter()
            .filter(|(_, t)| t.proposed)
            .map(|(id, _)| id.as_str())
    }

    /// Marks failures; proposes the death when they are enough.
    fn fail(&mut self, who: &str, n: u32, needed: u32, events: &mut Vec<FightEvent>) {
        let track = self.dying.entry(who.into()).or_default();
        if track.proposed {
            return;
        }
        track.failures = (track.failures + n).min(needed);
        track.stable = false;
        if track.failures >= needed {
            track.proposed = true;
            events.push(FightEvent::DeathProposed { who: who.into() });
        }
    }

    /// After an action: a hit on a dying character marks failures (two
    /// on a critical), a heal clears their boxes.
    pub(super) fn track_dying(&mut self, system: &RuleSystem, events: &mut Vec<FightEvent>) {
        let Some((_, _, needed)) = rule(system) else {
            return;
        };
        let mut critical_on: Option<String> = None;
        let mut marks: Vec<(String, u32)> = Vec::new();
        let mut healed: Vec<String> = Vec::new();
        let mut fell: Vec<String> = Vec::new();
        for e in events.iter() {
            let FightEvent::Rules { event } = e else {
                continue;
            };
            match event {
                Event::Roll {
                    purpose: RollPurpose::Attack,
                    against,
                    breakdown,
                    ..
                } => {
                    critical_on = against
                        .clone()
                        .filter(|_| breakdown.band == Some(OutcomeBand::CriticalSuccess));
                }
                Event::Damaged {
                    target, hp_before, ..
                } if *hp_before == 0 && self.dying_character(system, target) => {
                    let n = if critical_on.as_deref() == Some(target) {
                        2
                    } else {
                        1
                    };
                    marks.push((target.clone(), n));
                }
                Event::KnockedOut { target } if self.dying_character(system, target) => {
                    fell.push(target.clone());
                }
                Event::Revived { target } => healed.push(target.clone()),
                _ => {}
            }
        }
        for who in fell {
            self.dying.entry(who).or_default();
        }
        for (who, n) in marks {
            if !self.in_fight(&who) {
                continue;
            }
            self.fail(&who, n, needed, events);
            let failures = self.dying.get(&who).map_or(0, |t| t.failures);
            events.push(FightEvent::DeathFailure { who, failures });
        }
        for who in healed {
            self.dying.remove(&who);
        }
    }

    /// When the fight ends, those still dying stop rolling: stabilised
    /// by their friends when the party won, else their death is
    /// proposed to the GM (INTERPRETATION: no one is left to tend them;
    /// the GM may spare them).
    pub(super) fn settle_dying_at_end(&mut self, party_won: bool, events: &mut Vec<FightEvent>) {
        for (who, t) in &mut self.dying {
            if t.stable || t.proposed || self.standing.get(who) != Some(&Standing::InFight) {
                continue;
            }
            if party_won {
                t.stable = true;
                events.push(FightEvent::Stabilised { who: who.clone() });
            } else {
                t.proposed = true;
                events.push(FightEvent::DeathProposed { who: who.clone() });
            }
        }
    }

    /// The dying active character rolls their death save; their turn
    /// then ends.
    ///
    /// # Errors
    ///
    /// The turn is not theirs, or no save is due.
    pub fn death_save(
        &self,
        system: &RuleSystem,
        who: &str,
        dice: &mut dyn DiceSource,
    ) -> Result<Step, CombatRefusal> {
        self.open_turn(who)?;
        let Some((difficulty, needed_ok, needed_ko)) =
            rule(system).filter(|_| self.save_due(system, who))
        else {
            return Err(CombatRefusal::NoDeathSaveDue);
        };
        let die = &system.check.dice;
        let natural = dice.roll(die.faces);
        let mut next = self.clone();
        let mut events = Vec::new();
        let track = next.dying.entry(who.into()).or_default();
        let revived = natural == die.faces;
        if natural == 1 {
            track.failures += 2;
        } else if !revived {
            let total = i32::try_from(natural).unwrap_or(i32::MAX) + die.modifier;
            if total >= difficulty {
                track.successes += 1;
            } else {
                track.failures += 1;
            }
        }
        track.failures = track.failures.min(needed_ko);
        events.push(FightEvent::DeathSave {
            who: who.into(),
            die: die.to_string(),
            natural,
            difficulty,
            successes: track.successes,
            failures: track.failures,
        });
        if revived {
            let mut rules_events = Vec::new();
            conditions::gain_hp(system, &mut next.scene, who, 1, &mut rules_events);
            events.extend(
                rules_events
                    .into_iter()
                    .map(|event| FightEvent::Rules { event }),
            );
            next.dying.remove(who);
        } else if track.failures >= needed_ko {
            track.proposed = true;
            events.push(FightEvent::DeathProposed { who: who.into() });
        } else if track.successes >= needed_ok {
            track.stable = true;
            events.push(FightEvent::Stabilised { who: who.into() });
        }
        let engine = |e: conditions::TurnError| Refusal::Engine {
            detail: format!("{e:?}"),
        };
        next.close_turn(system, who, dice, &mut events)
            .map_err(engine)?;
        let current = next.turn;
        next.advance(system, Some(current), dice, &mut events)
            .map_err(engine)?;
        Ok(Step {
            fight: next,
            events,
        })
    }

    /// The GM's word on a death the engine proposed — also once the
    /// fight is over.
    ///
    /// # Errors
    ///
    /// No death is proposed for `who`.
    pub fn decide_death(&self, who: &str, call: DeathCall) -> Result<Step, CombatRefusal> {
        if !self.dying.get(who).is_some_and(|t| t.proposed) {
            return Err(CombatRefusal::NoDeathProposed);
        }
        let mut next = self.clone();
        let mut events = Vec::new();
        match call {
            DeathCall::Die => {
                next.dying.remove(who);
                next.leave(who, Standing::Dead);
                events.push(FightEvent::Died { who: who.into() });
                next.check_end(&mut events);
            }
            DeathCall::Spare => {
                next.dying.insert(
                    who.into(),
                    DeathTrack {
                        stable: true,
                        ..DeathTrack::default()
                    },
                );
                events.push(FightEvent::Spared { who: who.into() });
            }
        }
        Ok(Step {
            fight: next,
            events,
        })
    }
}
