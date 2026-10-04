//! The rule-system lint on the two witness worlds (`engine/lint-rule-system`).
//! Their drafts keep the defects Romain's games revealed on purpose: a
//! lint that misses them is broken (`MEMORY.md` §6). Each cleaned-up
//! copy fixes them in the file, as Romain would, and must report none.

use std::path::PathBuf;

use promptus_shared::rules::lint::{Issue, Severity};
use promptus_shared::rules::{BalanceParams, RuleSystem, balance_report, lint, lint_with};

fn text(id: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../content/rules/{id}/v1.yaml")),
    )
    .unwrap()
}

fn load(yaml: &str) -> RuleSystem {
    RuleSystem::from_yaml(yaml).unwrap_or_else(|e| panic!("does not load:\n{e}"))
}

/// The file with these edits — each must hit.
fn edit(id: &str, edits: &[(&str, &str)]) -> String {
    let mut t = text(id);
    for (from, to) in edits {
        assert!(t.contains(from), "`{from}` is not in {id}");
        t = t.replace(from, to);
    }
    t
}

fn has(issues: &[Issue], code: &str, path: &str) -> bool {
    issues.iter().any(|i| i.code == code && i.path == path)
}

fn codes(issues: &[Issue]) -> Vec<&str> {
    issues.iter().map(|i| i.code).collect()
}

fn assert_reports(issues: &[Issue], expected: &[(&str, &str)]) {
    for (code, path) in expected {
        assert!(
            has(issues, code, path),
            "missing {code} at {path}; got:\n{:#?}",
            issues
                .iter()
                .map(|i| format!("{} {}", i.code, i.path))
                .collect::<Vec<_>>()
        );
    }
}

/// The defects of the Corsaires draft, as the ticket lists them.
const CORSAIRES_DEFECTS: &[(&str, &str)] = &[
    // Two damage models: fixed for players, dice for NPCs and the shop.
    (
        "DAMAGE_MODEL_MIXED",
        "adversaries[gueule_rouge].actions[gueule_rouge_sabre].tags[0]",
    ),
    (
        "DAMAGE_MODEL_MIXED",
        "items[pistolet_a_silex_marche_noir].action.tags[0]",
    ),
    ("PRECISION_NOT_APPLIED", "attack.precision"),
    ("PRIMARY_ABILITY_AMBIGUOUS", "attack.ability"),
    (
        "ADVERSARY_AC_OFF_TIER",
        "adversaries[garde_royal].armor_class",
    ),
    (
        "ADVERSARY_AC_OFF_TIER",
        "adversaries[gueule_rouge].armor_class",
    ),
    ("ABILITY_TOTAL_OUTLIER", "classes[canonnier].abilities"),
    // Beyond the ticket's list: what the draft's header also names.
    (
        "ADVERSARY_AC_FORMULA",
        "adversaries[jacquot_le_sourd].armor_class",
    ),
    ("UNDEFINED_TERM", "items[epee_de_bonne_facture].note"),
    ("UNDEFINED_TERM", "items[kit_de_soins].note"),
    ("NAME_DUPLICATE", "items[pistolet_a_silex_marche_noir].name"),
];

const CORSAIRES_FIXES: &[(&str, &str)] = &[
    // One damage model: the players' fixed damage everywhere.
    ("amount: 1d6+2 }", "amount: 5 }"),
    ("amount: 1d4+2 }", "amount: 4 }"),
    ("amount: 1d6+1 }", "amount: 4 }"),
    ("amount: 1d4+1 }", "amount: 3 }"),
    ("amount: 1d6 }", "amount: 3 }"),
    ("amount: 1d4 }", "amount: 2 }"),
    ("amount: 1d3 }", "amount: 2 }"),
    ("amount: 1d2 }", "amount: 1 }"),
    ("amount: 1d8 }", "amount: 4 }"),
    ("precision: not_applied", "precision: added_to_attack_roll"),
    ("ability: first_primary", "ability: best_primary"),
    (
        "    armor_class: 14\n",
        "    armor_class: 14\n    exception: \"Plastron d'uniforme, voulu.\"\n",
    ),
    (
        "CHA: 11 }\n    armor_class: 12\n    hit_points: 10",
        "CHA: 11 }\n    armor_class: 15\n    hit_points: 10",
    ),
    ("    armor_class: 9\n", "    armor_class: 8\n"),
    (
        "SAG: 15, CHA: 13 }\n    armor_class: 10",
        "SAG: 15, CHA: 13 }\n    armor_class: 9",
    ),
    (
        "SAG: 13, CHA: 8 }\n    armor_class: 10\n    hit_points: 7",
        "SAG: 13, CHA: 8 }\n    armor_class: 9\n    hit_points: 7",
    ),
    (
        "{ FOR: 12, DEX: 10, CON: 10, INT: 14, SAG: 9, CHA: 8 }",
        "{ FOR: 12, DEX: 10, CON: 10, INT: 14, SAG: 10, CHA: 8 }",
    ),
    // A comparison with an item that exists is fine.
    (
        "par rapport à une épée standard (l'épée standard n'est définie nulle part).",
        "par rapport au Sabre d'abordage.",
    ),
    (
        "+1 aux jets de soin, 3 utilisations — mais les soins ne se lancent pas.",
        "+1 PV à chaque soin reçu, 3 utilisations.",
    ),
    (
        "    name: Pistolet à silex\n    description: 1 tir",
        "    name: Pistolet du marché noir\n    description: 1 tir",
    ),
];

/// Codes that measure balance rather than a defect of the text.
const BALANCE: &[&str] = &["PROGRESSION_MAX_EARLY", "DAMAGE_PER_TURN_LOW"];

#[test]
fn the_corsaires_draft_reports_every_known_defect() {
    let issues = lint(&load(&text("corsaires")));
    assert_reports(&issues, CORSAIRES_DEFECTS);
    // Each issue speaks French to the GM.
    for i in &issues {
        assert!(i.message.as_deref().is_some_and(|m| !m.is_empty()));
    }
    let canonnier = issues
        .iter()
        .find(|i| i.code == "ABILITY_TOTAL_OUTLIER")
        .unwrap();
    assert!(canonnier.message.as_deref().unwrap().contains("63"));
    // Only the outlier: the seven classes at 64 are fine.
    assert_eq!(
        codes(&issues)
            .iter()
            .filter(|c| **c == "ABILITY_TOTAL_OUTLIER")
            .count(),
        1
    );
}

#[test]
fn a_cleaned_corsaires_reports_none_of_them() {
    let issues = lint(&load(&edit("corsaires", CORSAIRES_FIXES)));
    for (code, path) in CORSAIRES_DEFECTS {
        assert!(!has(&issues, code, path), "{code} at {path} still reported");
    }
    let left: Vec<_> = issues
        .iter()
        .filter(|i| !BALANCE.contains(&i.code))
        .collect();
    assert!(left.is_empty(), "unexpected: {left:#?}");
}

#[test]
fn the_brasier_draft_reports_the_missing_ground_combat_file() {
    let issues = lint(&load(&text("brasier")));
    assert_reports(
        &issues,
        &[
            ("REFERENCE_MISSING", "turn_contexts[vaisseau].references[0]"),
            ("PRECISION_NOT_APPLIED", "attack.precision"),
            ("TURN_CONTEXTS_DIFFER", "turn_contexts[vaisseau].limits[0]"),
            ("COOLDOWN_UNEXPLAINED", "cooldowns.note"),
        ],
    );
    // The note cites the same file: one issue, not two.
    assert_eq!(
        codes(&issues)
            .iter()
            .filter(|c| **c == "REFERENCE_MISSING")
            .count(),
        1
    );
    let missing = issues
        .iter()
        .find(|i| i.code == "REFERENCE_MISSING")
        .unwrap();
    assert_eq!(missing.severity, Severity::Error);
    // The ship's single attack may be intended: the GM confirms.
    let ship = issues
        .iter()
        .find(|i| i.code == "TURN_CONTEXTS_DIFFER")
        .unwrap();
    assert_eq!(ship.severity, Severity::Info);
    // One primary ability per class, 64 points each, flat damage only.
    for code in [
        "PRIMARY_ABILITY_AMBIGUOUS",
        "ABILITY_TOTAL_OUTLIER",
        "DAMAGE_MODEL_MIXED",
    ] {
        assert!(!codes(&issues).contains(&code), "{code} reported");
    }
}

#[test]
fn a_cleaned_brasier_reports_none_of_them() {
    let fixed = edit(
        "brasier",
        &[
            ("    references: [Combat_Sol.md]\n", ""),
            (" (voir `Combat_Sol.md`)", ""),
            ("precision: not_applied", "precision: added_to_attack_roll"),
            (
                "  meaning: skip_next_turns\n",
                "  meaning: skip_next_turns\n  note: \"« CD N » : inutilisable pendant vos N prochains tours.\"\n",
            ),
        ],
    );
    let issues = lint(&load(&fixed));
    let left: Vec<_> = issues
        .iter()
        .filter(|i| !BALANCE.contains(&i.code) && i.code != "TURN_CONTEXTS_DIFFER")
        .collect();
    assert!(left.is_empty(), "unexpected: {left:#?}");
}

#[test]
fn a_reference_listed_among_the_sources_resolves() {
    let issues = lint(&load(&edit(
        "brasier",
        &[(
            "  - dnd-save/DnD_07-06-2026/Combat_Vaisseau.md\n",
            "  - dnd-save/DnD_07-06-2026/Combat_Vaisseau.md\n  - dnd-save/DnD_07-06-2026/Combat_Sol.md\n",
        )],
    )));
    assert!(!codes(&issues).contains(&"REFERENCE_MISSING"));
}

#[test]
fn an_exception_silences_only_its_own_stat_block() {
    let issues = lint(&load(&edit(
        "corsaires",
        &[(
            "    armor_class: 14\n",
            "    armor_class: 14\n    exception: \"Plastron d'uniforme.\"\n",
        )],
    )));
    assert!(!has(
        &issues,
        "ADVERSARY_AC_OFF_TIER",
        "adversaries[garde_royal].armor_class"
    ));
    assert!(has(
        &issues,
        "ADVERSARY_AC_OFF_TIER",
        "adversaries[gueule_rouge].armor_class"
    ));
}

#[test]
fn an_empty_exception_does_not_load() {
    let err = RuleSystem::from_yaml(&edit(
        "corsaires",
        &[(
            "    armor_class: 14\n",
            "    armor_class: 14\n    exception: \"\"\n",
        )],
    ))
    .unwrap_err();
    assert!(
        err.0
            .iter()
            .any(|e| e.path == "adversaries[garde_royal].exception")
    );
}

fn class_dpt(s: &RuleSystem, class: &str, level: u32, target: usize) -> f64 {
    let report = balance_report(s, &BalanceParams::default());
    let c = report.classes.iter().find(|c| c.id == class).unwrap();
    c.damage_per_turn
        .iter()
        .find(|d| d.level == level)
        .unwrap()
        .per_target[target]
}

#[test]
fn damage_per_turn_follows_the_rules_as_written() {
    let s = load(&text("corsaires"));
    // Estocade: 3 damage, no cooldown, two attacks a turn, FOR +1 against
    // AC 10 — hits on 9 to 19 (11 faces), doubles on a 20.
    assert_eq!(
        class_dpt(&s, "bretteur", 1, 0),
        2.0 * 3.0 * (11.0 + 2.0) / 20.0
    );
    // Precision added to the roll: +2 more faces hit.
    let precise = load(&edit(
        "corsaires",
        &[("precision: not_applied", "precision: added_to_attack_roll")],
    ));
    assert_eq!(
        class_dpt(&precise, "bretteur", 1, 0),
        2.0 * 3.0 * (13.0 + 2.0) / 20.0
    );
    // Tir de silex, cooldown 1: once every other turn as read here,
    // every turn when the turn of use counts.
    let skip = class_dpt(&s, "canonnier", 1, 0);
    let counts = class_dpt(
        &load(&edit(
            "corsaires",
            &[("meaning: skip_next_turns", "meaning: turn_of_use_counts")],
        )),
        "canonnier",
        1,
        0,
    );
    assert_eq!(counts, 2.0 * skip);
    // Harder targets, less damage.
    assert!(class_dpt(&s, "bretteur", 1, 3) < class_dpt(&s, "bretteur", 1, 0));
}

#[test]
fn the_ship_turn_limits_the_damage_it_allows() {
    let s = load(&text("brasier"));
    let ground = balance_report(&s, &BalanceParams::default());
    let ship = balance_report(
        &s,
        &BalanceParams {
            context: Some("vaisseau".into()),
            ..BalanceParams::default()
        },
    );
    let pilot = |r: &promptus_shared::rules::BalanceReport| {
        r.classes
            .iter()
            .find(|c| c.id == "pilote")
            .unwrap()
            .damage_per_turn[0]
            .per_target[0]
    };
    assert_eq!(pilot(&ship) * 2.0, pilot(&ground));
}

#[test]
fn xp_runs_away_on_a_long_campaign_not_on_a_single_evening() {
    let s = load(&text("corsaires"));
    let one = BalanceParams {
        sessions: 1,
        ..BalanceParams::default()
    };
    assert!(!codes(&lint_with(&s, &one)).contains(&"PROGRESSION_MAX_EARLY"));
    let issues = lint(&s);
    assert!(has(&issues, "PROGRESSION_MAX_EARLY", "progression.levels"));

    let report = balance_report(&s, &BalanceParams::default());
    let bretteur = report.classes.iter().find(|c| c.id == "bretteur").unwrap();
    assert_eq!(bretteur.final_level, s.max_level());
    // Slower thresholds slow everyone down.
    let slow = load(&edit(
        "corsaires",
        &[
            ("{ level: 2, xp: 5 }", "{ level: 2, xp: 20 }"),
            ("{ level: 3, xp: 10 }", "{ level: 3, xp: 40 }"),
            ("{ level: 4, xp: 15 }", "{ level: 4, xp: 60 }"),
            ("{ level: 5, xp: 20 }", "{ level: 5, xp: 80 }"),
            ("{ level: 6, xp: 25 }", "{ level: 6, xp: 100 }"),
            ("{ level: 7, xp: 30 }", "{ level: 7, xp: 120 }"),
        ],
    ));
    assert!(!codes(&lint(&slow)).contains(&"PROGRESSION_MAX_EARLY"));
}

#[test]
fn the_weakest_damage_dealer_is_pointed_out() {
    // Both Canonniers: 4 damage every other turn, where the others hit
    // for 3 or 4 twice a turn.
    for id in ["corsaires", "brasier"] {
        let issues = lint(&load(&text(id)));
        assert!(
            has(&issues, "DAMAGE_PER_TURN_LOW", "classes[canonnier].actions"),
            "{id}"
        );
    }
}
