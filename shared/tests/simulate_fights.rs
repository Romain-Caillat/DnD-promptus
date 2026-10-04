//! The fight simulator (`engine/simulate-fights`) on the two witness
//! scenarios: reproducible reports, numbers that add up to what the
//! fights' logs say, rule variants that change the rules they name,
//! and a comparison that sees the effect of a rule change.

use std::path::PathBuf;

use promptus_shared::combat::FightEvent;
use promptus_shared::combat::simulate::{SimParams, compare, play, simulate, tally};
use promptus_shared::combat::{PolicyKind, Scenario};
use promptus_shared::maps::Map;
use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::events::Event;
use promptus_shared::rules::model::{CooldownMeaning, PrecisionRule, Tag};
use promptus_shared::rules::variant::BuildError;

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

struct Loaded {
    scenario: Scenario,
    rules: String,
    map: Map,
}

fn load(world: &str, id: &str) -> Loaded {
    let scenario =
        Scenario::from_yaml(&read(&format!("content/scenarios/{world}/{id}.yaml"))).unwrap();
    let rules = read(&format!(
        "content/rules/{}/v{}.yaml",
        scenario.rules, scenario.version
    ));
    let map = Map::from_yaml(&read(&format!(
        "content/maps/{world}/{}.yaml",
        scenario.map
    )))
    .unwrap();
    Loaded {
        scenario,
        rules,
        map,
    }
}

impl Loaded {
    fn system(&self, variant: Option<&str>) -> RuleSystem {
        let v = variant.map(|id| self.scenario.variant(id).expect("variant"));
        self.scenario.system(&self.rules, v).unwrap()
    }
}

fn dock() -> Loaded {
    load("corsaires", "bagarre-du-quai")
}

fn corridor() -> Loaded {
    load("brasier", "abordage-coursive")
}

#[test]
fn the_same_seed_gives_the_same_report() {
    for l in [dock(), corridor()] {
        let s = l.system(None);
        let p = SimParams::new(12, 40);
        let run = || {
            simulate(
                &s,
                &l.scenario,
                &l.map,
                PolicyKind::Focus,
                PolicyKind::Brawler,
                &p,
            )
            .unwrap()
        };
        let (a, b) = (run(), run());
        assert_eq!(a, b);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        let other = simulate(
            &s,
            &l.scenario,
            &l.map,
            PolicyKind::Focus,
            PolicyKind::Brawler,
            &SimParams::new(12, 41),
        )
        .unwrap();
        assert_ne!(a.groups, other.groups, "another seed, other fights");
    }
}

#[test]
fn the_report_adds_up_to_the_fights_logs() {
    for l in [dock(), corridor()] {
        let s = l.system(None);
        let p = SimParams::new(10, 3);
        let report = simulate(
            &s,
            &l.scenario,
            &l.map,
            PolicyKind::Brawler,
            PolicyKind::Brawler,
            &p,
        )
        .unwrap();
        assert_eq!(report.refusals, 0);
        let (mut xp, mut hp_lost, mut wins, mut rounds) = (0u64, 0u64, 0u32, 0u32);
        for i in 0..10 {
            let log = play(
                &s,
                &l.scenario,
                &l.map,
                PolicyKind::Brawler,
                PolicyKind::Brawler,
                3 + i,
                p.limits,
            )
            .unwrap();
            let end = log.fight.end.as_ref().unwrap();
            xp += end.xp.values().map(|x| u64::from(*x)).sum::<u64>();
            rounds += end.rounds;
            wins += u32::from(end.winner == Some(promptus_shared::rules::sheet::Side::Party));
            for e in log.events() {
                if let FightEvent::Rules {
                    event:
                        Event::Damaged {
                            hp_before,
                            hp_after,
                            ..
                        },
                } = e
                {
                    hp_lost += u64::try_from(hp_before - hp_after).unwrap();
                }
            }
            // One fight's minutes follow the time model from its turns.
            let t = tally(&log, &p.time);
            assert_eq!(
                t.seconds,
                p.time.setup_seconds
                    + t.party_turns * p.time.party_turn_seconds
                    + t.opposition_turns * p.time.opposition_turn_seconds
                    + t.idle_turns * p.time.idle_turn_seconds
            );
            assert!(t.party_turns > 0 && t.opposition_turns > 0);
        }
        let sum = |f: fn(&promptus_shared::combat::simulate::GroupTally) -> u64| {
            report.groups.iter().map(|g| f(&g.totals)).sum::<u64>()
        };
        assert_eq!(sum(|t| t.xp), xp);
        assert_eq!(sum(|t| t.damage_taken), hp_lost);
        assert_eq!(
            sum(|t| t.damage_dealt) + report.damage_from_conditions,
            hp_lost
        );
        assert_eq!(report.outcomes.party_wins, wins);
        assert!((report.rounds.mean - f64::from(rounds) / 10.0).abs() < 1e-9);
        assert!(report.rounds.min <= report.rounds.median);
        assert!(report.rounds.median <= report.rounds.max);
        // Every group of the scenario is there, sailors counted together.
        let marins = report
            .groups
            .iter()
            .find(|g| g.id == "marin_de_gueule_rouge" || g.id == "abordeur-vorr")
            .unwrap();
        assert_eq!(marins.members, 5);
    }
}

#[test]
fn variants_change_the_rules_they_name_and_nothing_else() {
    let d = dock();
    let base = d.system(None);
    assert_eq!(base.attack.precision, PrecisionRule::NotApplied);
    let precise = d.system(Some("precision-au-jet"));
    assert_eq!(precise.attack.precision, PrecisionRule::AddedToAttackRoll);
    assert_eq!(precise.cooldowns.meaning, base.cooldowns.meaning);

    let fixed = d.system(Some("degats-fixes-pnj"));
    let amounts = |s: &RuleSystem, block: &str| -> Vec<String> {
        s.adversary(block)
            .unwrap()
            .actions
            .iter()
            .flat_map(|a| &a.tags)
            .filter_map(|t| match t {
                Tag::Damage(d) => Some(d.amount.to_string()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(amounts(&base, "gueule_rouge"), ["1d6+2", "1d4+2"]);
    assert_eq!(amounts(&fixed, "gueule_rouge"), ["5", "4"]);
    assert_eq!(amounts(&fixed, "marin_de_gueule_rouge"), ["3", "2"]);
    assert_eq!(
        amounts(&fixed, "capitaine_morel"),
        amounts(&base, "capitaine_morel"),
        "untouched blocks stay as written"
    );

    let c = corridor();
    assert_eq!(
        c.system(None).cooldowns.meaning,
        CooldownMeaning::SkipNextTurns
    );
    assert_eq!(
        c.system(Some("cd-tour-d-usage")).cooldowns.meaning,
        CooldownMeaning::TurnOfUseCounts
    );
}

#[test]
fn a_variant_naming_something_absent_is_refused() {
    let d = dock();
    let mut v = d.scenario.variant("precision-au-jet").unwrap().clone();
    v.set.clear();
    v.set.insert(
        "adversaries[nobody].hit_points".into(),
        serde_yaml_ng::Value::from(3),
    );
    let e = d.scenario.system(&d.rules, Some(&v)).unwrap_err();
    assert!(matches!(e, BuildError::Edit(_)), "{e}");
    v.set.clear();
    v.set.insert("attack.precision".into(), "sometimes".into());
    assert!(matches!(
        d.scenario.system(&d.rules, Some(&v)).unwrap_err(),
        BuildError::Load(_)
    ));
}

#[test]
fn counting_precision_shows_in_the_comparison() {
    let d = dock();
    let p = SimParams::new(40, 1);
    let run = |variant| {
        simulate(
            &d.system(variant),
            &d.scenario,
            &d.map,
            PolicyKind::Brawler,
            PolicyKind::Brawler,
            &p,
        )
        .unwrap()
    };
    let (base, precise) = (run(None), run(Some("precision-au-jet")));
    let delta = compare(&base, &precise);
    // Precision only helps the classes whose cards carry it: they hit
    // more often, and each hit is one XP.
    let bretteur = delta.groups.iter().find(|g| g.id == "bretteur").unwrap();
    assert!(bretteur.hit_rate > 0.0, "{delta:?}");
    let party_xp: f64 = delta
        .groups
        .iter()
        .filter(|g| g.side == promptus_shared::rules::sheet::Side::Party)
        .map(|g| g.xp)
        .sum();
    assert!(party_xp > 0.0, "{delta:?}");
    // The adversaries carry no precision: their hit rate does not move
    // for that reason (only through who is left to hit).
    assert_eq!(compare(&base, &base).party_win_rate, 0.0);
}
