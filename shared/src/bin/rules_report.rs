//! `bun run rules-report` — the phase 1 report (`engine/simulate-fights`):
//! for each world, what the rule lint finds, the balance numbers, the
//! simulated fights of its scenarios and the effect of each rule variant
//! they try. Written in French, for Romain to evolve his rules before a
//! table sees them.
//!
//! Reads `content/rules/<world>/v<n>.yaml` (the highest version is
//! linted), `content/scenarios/<world>/*.yaml` and the maps they name in
//! `content/maps/<world>/`. The exit code is non-zero only when a file
//! does not load (or a variant or a scenario cannot be built); lint
//! findings never change it.
//!
//! Usage: `rules_report [--world corsaires|brasier|srd] [--n N] [--seed S]
//! [--json] [--markdown [FILE]] [--content DIR]`. `--markdown` also
//! writes the report to FILE (default `docs/rapport-phase-1.md`).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;

use promptus_shared::combat::simulate::{
    Delta, SimParams, SimReport, TimeModel, compare, simulate,
};
use promptus_shared::combat::{PolicyKind, Scenario};
use promptus_shared::issue::{Issue, Severity};
use promptus_shared::maps::Map;
use promptus_shared::rules::sheet::Side;
use promptus_shared::rules::variant::Variant;
use promptus_shared::rules::{BalanceParams, BalanceReport, RuleSystem, balance_report, lint};

#[derive(Debug, Clone, Default)]
struct Options {
    content: PathBuf,
    world: Option<String>,
    fights: Option<u32>,
    seed: Option<u64>,
    json: bool,
    markdown: Option<PathBuf>,
}

fn repo() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    std::fs::canonicalize(&path).unwrap_or(path)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut o = Options {
        content: repo().join("content"),
        ..Options::default()
    };
    let mut args = args.into_iter().peekable();
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} attend une valeur"));
        match a.as_str() {
            "--world" => o.world = Some(value("--world")?),
            "--n" => {
                o.fights = Some(value("--n")?.parse().map_err(|e| format!("--n : {e}"))?);
            }
            "--seed" => {
                o.seed = Some(
                    value("--seed")?
                        .parse()
                        .map_err(|e| format!("--seed : {e}"))?,
                );
            }
            "--content" => o.content = PathBuf::from(value("--content")?),
            "--json" => o.json = true,
            "--markdown" => {
                let path = match args.peek() {
                    Some(p) if !p.starts_with("--") => {
                        PathBuf::from(args.next().unwrap_or_default())
                    }
                    _ => repo().join("docs/rapport-phase-1.md"),
                };
                o.markdown = Some(path);
            }
            other => return Err(format!("option inconnue : {other}")),
        }
    }
    Ok(o)
}

// ------------------------------------------------------------ the report

#[derive(Debug, Serialize)]
struct Report {
    fights: Option<u32>,
    seed: Option<u64>,
    time_model: TimeModel,
    worlds: Vec<WorldReport>,
}

#[derive(Debug, Serialize)]
struct WorldReport {
    world: String,
    rules: String,
    version: u32,
    name: String,
    file: String,
    lint: Vec<Issue>,
    balance: BalanceReport,
    scenarios: Vec<ScenarioReport>,
}

#[derive(Debug, Serialize)]
struct ScenarioReport {
    id: String,
    name: String,
    description: String,
    source: String,
    file: String,
    map: String,
    rules: String,
    version: u32,
    /// Stat blocks the scenario adds because the rules lack them.
    stand_ins: Vec<String>,
    fights: u32,
    seed: u64,
    runs: Vec<SimReport>,
    comparisons: Vec<ComparisonReport>,
}

#[derive(Debug, Serialize)]
struct ComparisonReport {
    variant: Variant,
    /// Lint findings (`CODE path`) the variant removes and adds.
    lint_removed: Vec<String>,
    lint_added: Vec<String>,
    by_policy: Vec<PolicyComparison>,
}

#[derive(Debug, Serialize)]
struct PolicyComparison {
    party_policy: PolicyKind,
    opposition_policy: PolicyKind,
    variant: SimReport,
    delta: Delta,
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()))
}

/// Sub-directories of `dir`, sorted.
fn dirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    out.sort();
    out
}

/// YAML files of `dir`, sorted.
fn yaml_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml" || e == "yml"))
        .collect();
    out.sort();
    out
}

/// The highest `v<n>.yaml` of a rules directory.
fn latest_rules(dir: &Path) -> Option<(u32, PathBuf)> {
    yaml_files(dir)
        .into_iter()
        .filter_map(|p| {
            let n = p.file_stem()?.to_str()?.strip_prefix('v')?.parse().ok()?;
            Some((n, p))
        })
        .max_by_key(|(n, _)| *n)
}

fn finding(i: &Issue) -> String {
    format!("{} {}", i.code, i.path)
}

fn build(o: &Options) -> Result<Report, Vec<String>> {
    let rel = |p: &Path| {
        p.strip_prefix(o.content.parent().unwrap_or(&o.content))
            .unwrap_or(p)
            .display()
            .to_string()
    };
    let mut errors = Vec::new();
    let mut worlds = Vec::new();
    for (world, dir) in dirs(&o.content.join("rules")) {
        if o.world.as_ref().is_some_and(|w| *w != world) {
            continue;
        }
        let Some((_, path)) = latest_rules(&dir) else {
            continue;
        };
        let system = match read(&path)
            .and_then(|t| RuleSystem::from_yaml(&t).map_err(|e| format!("{} : {e}", rel(&path))))
        {
            Ok(s) => s,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        let mut scenarios = Vec::new();
        for file in yaml_files(&o.content.join("scenarios").join(&world)) {
            match scenario(o, &world, &file) {
                Ok(mut s) => {
                    s.file = rel(&file);
                    scenarios.push(s);
                }
                Err(e) => errors.push(format!("{} : {e}", rel(&file))),
            }
        }
        worlds.push(WorldReport {
            world,
            rules: system.id.clone(),
            version: system.version,
            name: system.name.clone(),
            file: rel(&path),
            lint: lint(&system),
            balance: balance_report(&system, &BalanceParams::default()),
            scenarios,
        });
    }
    if let Some(w) = &o.world
        && worlds.iter().all(|r| &r.world != w)
        && errors.is_empty()
    {
        errors.push(format!("aucun monde « {w} » dans {}", o.content.display()));
    }
    if errors.is_empty() {
        Ok(Report {
            fights: o.fights,
            seed: o.seed,
            time_model: TimeModel::default(),
            worlds,
        })
    } else {
        Err(errors)
    }
}

/// Loads one scenario and plays everything it asks for.
fn scenario(o: &Options, world: &str, file: &Path) -> Result<ScenarioReport, String> {
    let sc = Scenario::from_yaml(&read(file)?).map_err(|e| e.to_string())?;
    let rules_path = o
        .content
        .join("rules")
        .join(&sc.rules)
        .join(format!("v{}.yaml", sc.version));
    let rules = read(&rules_path)?;
    let map_path = o
        .content
        .join("maps")
        .join(world)
        .join(format!("{}.yaml", sc.map));
    let map =
        Map::from_yaml(&read(&map_path)?).map_err(|e| format!("{} : {e}", map_path.display()))?;
    let draft = RuleSystem::from_yaml(&rules).map_err(|e| e.to_string())?;
    let base = sc.system(&rules, None).map_err(|e| e.to_string())?;
    let stand_ins = base
        .adversaries
        .iter()
        .filter(|a| draft.adversary(&a.id).is_none())
        .map(|a| a.id.clone())
        .collect();
    let mut params = SimParams::new(
        o.fights.unwrap_or(sc.simulation.fights),
        o.seed.unwrap_or(sc.simulation.seed),
    );
    params.time = TimeModel::default();
    let pairs: Vec<(PolicyKind, PolicyKind)> = sc
        .party
        .policies
        .iter()
        .flat_map(|p| sc.opposition.policies.iter().map(move |q| (*p, *q)))
        .collect();
    let run = |system: &RuleSystem, (p, q): (PolicyKind, PolicyKind)| {
        simulate(system, &sc, &map, p, q, &params).map_err(|e| e.to_string())
    };
    let runs = pairs
        .iter()
        .map(|pair| run(&base, *pair))
        .collect::<Result<Vec<_>, _>>()?;
    let base_lint: BTreeSet<String> = lint(&base).iter().map(finding).collect();
    let mut comparisons = Vec::new();
    for v in &sc.variants {
        let system = sc
            .system(&rules, Some(v))
            .map_err(|e| format!("variante {} : {e}", v.id))?;
        let lint_after: BTreeSet<String> = lint(&system).iter().map(finding).collect();
        let mut by_policy = Vec::new();
        for (pair, base_run) in pairs.iter().zip(&runs) {
            let variant = run(&system, *pair)?;
            by_policy.push(PolicyComparison {
                party_policy: pair.0,
                opposition_policy: pair.1,
                delta: compare(base_run, &variant),
                variant,
            });
        }
        comparisons.push(ComparisonReport {
            variant: v.clone(),
            lint_removed: base_lint.difference(&lint_after).cloned().collect(),
            lint_added: lint_after.difference(&base_lint).cloned().collect(),
            by_policy,
        });
    }
    Ok(ScenarioReport {
        id: sc.id.clone(),
        name: sc.name.clone(),
        description: sc.description.trim().to_string(),
        source: sc.source.clone(),
        file: String::new(),
        map: sc.map.clone(),
        rules: sc.rules.clone(),
        version: sc.version,
        stand_ins,
        fights: params.fights,
        seed: params.seed,
        runs,
        comparisons,
    })
}

// ------------------------------------------------------------ French text

/// A number the French way: decimal comma.
fn fr(x: f64, decimals: usize) -> String {
    format!("{x:.decimals$}").replace('.', ",")
}

fn signed(x: f64, decimals: usize) -> String {
    let s = fr(x.abs(), decimals);
    if s.trim_start_matches(['0', ',']).is_empty() {
        format!("±{s}")
    } else if x > 0.0 {
        format!("+{s}")
    } else {
        format!("−{s}")
    }
}

fn pct(x: f64) -> String {
    format!("{} %", fr(x * 100.0, 0))
}

fn signed_pct(x: f64) -> String {
    format!("{} pts", signed(x * 100.0, 0))
}

fn policy_name(p: PolicyKind) -> &'static str {
    match p {
        PolicyKind::Brawler => "bagarreur",
        PolicyKind::Focus => "concentré",
    }
}

fn severity_name(s: Severity) -> &'static str {
    match s {
        Severity::Error => "Erreurs",
        Severity::Warning => "Avertissements",
        Severity::Info => "À confirmer",
    }
}

fn markdown(r: &Report) -> String {
    let mut out = String::new();
    let t = &r.time_model;
    let _ = writeln!(out, "# Rapport de la phase 1 — les règles à l'épreuve\n");
    let _ = writeln!(
        out,
        "Produit par `bun run rules-report` (`shared/src/bin/rules_report.rs`). Pour \
         chaque monde : ce que le contrôle trouve dans le brouillon de règles, les \
         chiffres d'équilibre, les combats simulés de ses scénarios \
         (`content/scenarios/`), puis l'effet de chaque variante de règle, jouée sur \
         les mêmes graines. Rien ici ne bloque : le MJ décide.\n"
    );
    let _ = writeln!(out, "**Lire les simulations.**\n");
    let _ = writeln!(
        out,
        "- Les deux camps sont joués par des tactiques de référence, pas par une IA. \
         *Bagarreur* : soigne un allié à terre, sinon frappe l'ennemi le plus proche, \
         sinon avance droit sur lui. *Concentré* : pareil, mais frappe l'ennemi le plus \
         faible à sa portée, se place à la case la moins chère d'où il peut tirer, et ne \
         s'arrête jamais dans une porte. Les adversaires jouent en bagarreurs, avec le \
         moral que le scénario leur donne (qui fuit, quand).\n\
         - Durée estimée (INTERPRÉTATION, à recaler sur un combat chronométré) : {} min \
         de mise en place, {} s par tour de personnage, {} s par tour d'adversaire, {} s \
         par tour vide (KO, rien à faire).\n\
         - Les chiffres par groupe sont des moyennes par combat ; un groupe d'adversaires \
         additionne ses membres (5 marins = un groupe). « Touche » = jets d'attaque \
         réussis sur jets d'attaque lancés (— : aucune attaque, la classe n'a que du \
         soin ou du soutien à ce niveau).\n",
        fr(f64::from(t.setup_seconds) / 60.0, 0),
        t.party_turn_seconds,
        t.opposition_turn_seconds,
        t.idle_turn_seconds,
    );
    for w in &r.worlds {
        world_section(&mut out, w);
    }
    out
}

fn world_section(out: &mut String, w: &WorldReport) {
    let _ = writeln!(out, "## {} — règles `{}` v{}\n", w.name, w.rules, w.version);
    let _ = writeln!(out, "Fichier : `{}`.\n", w.file);

    let _ = writeln!(out, "### Contrôle des règles\n");
    if w.lint.is_empty() {
        let _ = writeln!(out, "Rien à signaler.\n");
    }
    for sev in [Severity::Error, Severity::Warning, Severity::Info] {
        let found: Vec<&Issue> = w.lint.iter().filter(|i| i.severity == sev).collect();
        if found.is_empty() {
            continue;
        }
        let _ = writeln!(out, "**{} ({})**\n", severity_name(sev), found.len());
        for i in found {
            let text = i.message.as_deref().unwrap_or(&i.detail);
            let _ = writeln!(out, "- `{}` · `{}` — {text}", i.code, i.path);
        }
        let _ = writeln!(out);
    }

    balance_section(out, &w.balance);

    let _ = writeln!(out, "### Combats simulés\n");
    if w.scenarios.is_empty() {
        let _ = writeln!(
            out,
            "Aucun scénario dans `content/scenarios/{}/`.\n",
            w.world
        );
    }
    for s in &w.scenarios {
        scenario_section(out, s);
    }
}

fn balance_section(out: &mut String, b: &BalanceReport) {
    let _ = writeln!(out, "### Équilibre (calcul attendu, pas une simulation)\n");
    let targets: Vec<String> = b
        .targets
        .iter()
        .map(|t| format!("{} (CA {})", t.label, t.armor_class))
        .collect();
    let _ = writeln!(
        out,
        "Campagne type : {} sessions de {} combats de {} rounds et {} jets hors combat ; \
         contexte `{}` ({} actions par tour). Cibles : {}. XP calculée contre CA {}, \
         jets contre {} ; total de caractéristiques médian {}.\n",
        b.params.sessions,
        b.params.fights_per_session,
        b.params.rounds_per_fight,
        b.params.checks_per_session,
        b.context,
        b.actions_per_turn,
        targets.join(", "),
        b.xp_target,
        b.check_difficulty,
        b.ability_total_median,
    );
    let labels: Vec<&str> = b.targets.iter().map(|t| t.label.as_str()).collect();
    let _ = writeln!(
        out,
        "| Classe | Total caract. | Dégâts/tour niv. 1 ({}) | Dégâts/tour niv. 7 | XP en fin de campagne | Niveau final | Niveau max dès |",
        labels.join(" / ")
    );
    let _ = writeln!(out, "|---|---:|---|---|---:|---:|---|");
    for c in &b.classes {
        let at = |level: u32| {
            c.damage_per_turn
                .iter()
                .find(|d| d.level == level)
                .map(|d| {
                    d.per_target
                        .iter()
                        .map(|x| fr(*x, 1))
                        .collect::<Vec<_>>()
                        .join(" / ")
                })
                .unwrap_or_default()
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} |",
            c.name,
            c.ability_total,
            at(1),
            at(7),
            fr(c.sessions.last().map_or(0.0, |s| s.total_xp), 0),
            c.final_level,
            c.max_level_after_session
                .map_or_else(|| "jamais".into(), |n| format!("session {n}")),
        );
    }
    let _ = writeln!(out);
}

fn summary_row(r: &SimReport) -> String {
    let o = &r.outcomes;
    format!(
        "| {} | {} ({}/{}) | {} | {} | {} ({}–{}) | {} ({}–{}) | {} |",
        policy_name(r.party_policy),
        pct(r.party_win_rate),
        o.party_wins,
        r.params.fights,
        o.opposition_wins,
        o.draws + o.stopped,
        fr(r.rounds.mean, 1),
        fr(r.rounds.min, 0),
        fr(r.rounds.max, 0),
        fr(r.minutes.mean, 0),
        fr(r.minutes.min, 0),
        fr(r.minutes.max, 0),
        r.refusals,
    )
}

const SUMMARY_HEAD: &str = "| Tactique des PJ | Victoires PJ | Défaites | Nuls ou arrêtés | Rounds moy. (min–max) | Minutes estimées moy. (min–max) | Refus du moteur |\n|---|---:|---:|---:|---|---|---:|";

fn scenario_section(out: &mut String, s: &ScenarioReport) {
    let _ = writeln!(out, "#### {}\n", s.name);
    let _ = writeln!(out, "{}\n", s.description);
    let _ = writeln!(
        out,
        "Scénario `{}`, carte `{}`, règles `{}` v{} ; source : {}. {} combats par \
         tactique, graines {} à {}.\n",
        s.file,
        s.map,
        s.rules,
        s.version,
        s.source,
        s.fights,
        s.seed,
        s.seed + u64::from(s.fights.saturating_sub(1)),
    );
    if !s.stand_ins.is_empty() {
        let ids: Vec<String> = s.stand_ins.iter().map(|i| format!("`{i}`")).collect();
        let _ = writeln!(
            out,
            "> **Fiches d'adversaire hors règles.** Le système de règles n'a pas {} : le \
             scénario les définit lui-même, sans toucher au fichier de règles (voir le \
             commentaire du scénario pour leur origine et ce qui n'est pas simulé).\n",
            ids.join(", ")
        );
    }
    let _ = writeln!(out, "{SUMMARY_HEAD}");
    for r in &s.runs {
        let _ = writeln!(out, "{}", summary_row(r));
    }
    let _ = writeln!(out);
    for r in &s.runs {
        let _ = writeln!(
            out,
            "**Par groupe — PJ {}** (tours par combat : {} de PJ, {} d'adversaires, {} vides)\n",
            policy_name(r.party_policy),
            fr(r.party_turns, 1),
            fr(r.opposition_turns, 1),
            fr(r.idle_turns, 1),
        );
        let _ = writeln!(
            out,
            "| Groupe | Nb | Dégâts infligés | Dégâts reçus | Touche | KO | Hors scène | Fuites | Vaincus | XP |"
        );
        let _ = writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
        for g in &r.groups {
            let m = &g.per_fight;
            let xp = match g.side {
                Side::Party => fr(m.xp, 1),
                Side::Opposition => "—".into(),
            };
            let hit = if g.totals.attacks == 0 {
                "—".to_string()
            } else {
                pct(m.hit_rate)
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {xp} |",
                g.name,
                g.members,
                fr(m.damage_dealt, 1),
                fr(m.damage_taken, 1),
                hit,
                fr(m.knocked_out, 2),
                fr(m.out_of_scene, 2),
                fr(m.fled, 2),
                fr(m.defeated, 2),
            );
        }
        if r.damage_from_conditions > 0 {
            let _ = writeln!(
                out,
                "\nDégâts des états (fin de tour) : {} par combat.",
                fr(
                    r.damage_from_conditions as f64 / f64::from(r.params.fights.max(1)),
                    1
                )
            );
        }
        let _ = writeln!(out);
    }
    for c in &s.comparisons {
        comparison_section(out, s, c);
    }
}

fn comparison_section(out: &mut String, s: &ScenarioReport, c: &ComparisonReport) {
    let v = &c.variant;
    let _ = writeln!(out, "##### Variante : {}\n", v.name);
    if !v.description.trim().is_empty() {
        let _ = writeln!(out, "{}\n", v.description.trim());
    }
    let _ = writeln!(out, "Modifications appliquées au brouillon, en mémoire :\n");
    for (path, value) in &v.set {
        let path = path.as_str().unwrap_or("?");
        let value = serde_yaml_ng::to_string(value).unwrap_or_default();
        let _ = writeln!(out, "- `{path}` → `{}`", value.trim());
    }
    let _ = writeln!(out);
    if c.lint_removed.is_empty() && c.lint_added.is_empty() {
        let _ = writeln!(out, "Contrôle des règles : inchangé.\n");
    } else {
        let list = |l: &[String]| {
            if l.is_empty() {
                "aucun".to_string()
            } else {
                l.iter()
                    .map(|x| format!("`{x}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        };
        let _ = writeln!(
            out,
            "Contrôle des règles : disparaît {} ; apparaît {}.\n",
            list(&c.lint_removed),
            list(&c.lint_added)
        );
    }
    let _ = writeln!(
        out,
        "| Tactique des PJ | Victoires PJ (brouillon → variante) | Rounds moy. | Minutes moy. |"
    );
    let _ = writeln!(out, "|---|---|---|---|");
    for p in &c.by_policy {
        let base = s.runs.iter().find(|r| {
            r.party_policy == p.party_policy && r.opposition_policy == p.opposition_policy
        });
        let Some(base) = base else { continue };
        let _ = writeln!(
            out,
            "| {} | {} → {} ({}) | {} → {} ({}) | {} → {} ({}) |",
            policy_name(p.party_policy),
            pct(base.party_win_rate),
            pct(p.variant.party_win_rate),
            signed_pct(p.delta.party_win_rate),
            fr(base.rounds.mean, 1),
            fr(p.variant.rounds.mean, 1),
            signed(p.delta.rounds, 1),
            fr(base.minutes.mean, 0),
            fr(p.variant.minutes.mean, 0),
            signed(p.delta.minutes, 1),
        );
    }
    let _ = writeln!(out);
    for p in &c.by_policy {
        let _ = writeln!(
            out,
            "Écarts par groupe, PJ {} (variante − brouillon, par combat) :\n",
            policy_name(p.party_policy)
        );
        let _ = writeln!(
            out,
            "| Groupe | Dégâts infligés | Dégâts reçus | Touche | XP |"
        );
        let _ = writeln!(out, "|---|---:|---:|---:|---:|");
        for g in &p.delta.groups {
            let xp = match g.side {
                Side::Party => signed(g.xp, 2),
                Side::Opposition => "—".into(),
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {xp} |",
                g.name,
                signed(g.damage_dealt, 2),
                signed(g.damage_taken, 2),
                signed_pct(g.hit_rate),
            );
        }
        let _ = writeln!(out);
    }
}

fn main() -> ExitCode {
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let report = match build(&options) {
        Ok(r) => r,
        Err(errors) => {
            eprintln!("Fichiers qui ne se chargent pas :");
            for e in errors {
                eprintln!("  - {e}");
            }
            return ExitCode::FAILURE;
        }
    };
    let text = markdown(&report);
    if options.json {
        match serde_json::to_string_pretty(&report) {
            Ok(j) => println!("{j}"),
            Err(e) => {
                eprintln!("JSON : {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        print!("{text}");
    }
    if let Some(path) = &options.markdown {
        if let Err(e) = std::fs::write(path, &text) {
            eprintln!("{} : {e}", path.display());
            return ExitCode::FAILURE;
        }
        eprintln!("Rapport écrit dans {}", path.display());
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(extra: &[&str]) -> Options {
        let mut args = vec!["--n".to_string(), "3".to_string()];
        args.extend(extra.iter().map(|s| s.to_string()));
        parse_args(args).unwrap()
    }

    #[test]
    fn the_report_covers_both_worlds_and_the_srd_end_to_end() {
        let report = build(&options(&[])).unwrap();
        let worlds: Vec<&str> = report.worlds.iter().map(|w| w.world.as_str()).collect();
        assert_eq!(worlds, ["brasier", "corsaires", "srd"]);
        for w in &report.worlds {
            if w.world != "srd" {
                assert!(!w.lint.is_empty(), "both drafts have known defects");
            }
            assert!(!w.scenarios.is_empty(), "{}", w.world);
            for s in &w.scenarios {
                assert!(!s.comparisons.is_empty(), "{}", s.id);
                for r in &s.runs {
                    assert_eq!(r.params.fights, 3);
                    assert_eq!(r.refusals, 0, "{}", s.id);
                }
            }
        }
        let text = markdown(&report);
        for needle in [
            "La bagarre du quai",
            "L'abordage de la coursive",
            "L'embuscade des gobelins",
            "DAMAGE_MODEL_MIXED",
            "REFERENCE_MISSING",
            "La précision compte au jet d'attaque",
            "« CD 1 » = pas deux fois dans le même tour",
            "Un seul modèle de dégâts",
        ] {
            assert!(text.contains(needle), "missing {needle}");
        }
        // The Corsaires' precision variant clears the lint's complaint.
        let corsaires = report
            .worlds
            .iter()
            .find(|w| w.world == "corsaires")
            .unwrap();
        let precision = corsaires.scenarios[0]
            .comparisons
            .iter()
            .find(|c| c.variant.id == "precision-au-jet")
            .unwrap();
        assert!(
            precision
                .lint_removed
                .iter()
                .any(|f| f.starts_with("PRECISION_NOT_APPLIED"))
        );
        let json = serde_json::to_value(&report).unwrap();
        assert!(json["worlds"][0]["scenarios"][0]["runs"][0]["party_win_rate"].is_number());
    }

    #[test]
    fn one_world_can_be_asked_for_and_an_unknown_one_fails() {
        let report = build(&options(&["--world", "corsaires", "--seed", "9"])).unwrap();
        assert_eq!(report.worlds.len(), 1);
        assert_eq!(report.worlds[0].scenarios[0].seed, 9);
        assert!(build(&options(&["--world", "atlantide"])).is_err());
        assert!(parse_args(["--bogus".to_string()]).is_err());
    }
}
