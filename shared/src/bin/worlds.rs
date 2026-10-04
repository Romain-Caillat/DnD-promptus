//! `bun run worlds` — loads every rule system, map and campaign under
//! `content/` and reports, world by world, what the checks find
//! (`campaign/rewrite-two-worlds`).
//!
//! Layout: `content/rules/<world>/v<n>.yaml`, `content/maps/<world>/*.yaml`,
//! `content/campaigns/<world>/*.yaml`. Fixtures (`content/fixtures/`)
//! are test data and are not read.
//!
//! The exit code is non-zero only when a file does not load (syntax,
//! shape, a rule system or map the engine refuses). Checks never block:
//! their errors and warnings are printed, the exit code stays 0.
//!
//! Usage: `worlds [content-dir]` (default: the repository's `content/`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use promptus_shared::maps::{Map, Scale};
use promptus_shared::rules::RuleSystem;
use promptus_shared::story::{Campaign, Issue, Library, Severity, from_yaml, validate_with};

/// Files of one world, by kind.
#[derive(Default)]
struct World {
    rules: Vec<(PathBuf, RuleSystem)>,
    maps: Vec<(PathBuf, Map)>,
    campaigns: Vec<(PathBuf, Campaign)>,
}

/// YAML files of `dir/<world>/`, grouped by world, sorted.
fn yaml_files(dir: &Path) -> BTreeMap<String, Vec<PathBuf>> {
    let mut out: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    let Ok(worlds) = std::fs::read_dir(dir) else {
        return out;
    };
    for world in worlds.flatten() {
        let Ok(files) = std::fs::read_dir(world.path()) else {
            continue;
        };
        let name = world.file_name().to_string_lossy().into_owned();
        for f in files.flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                out.entry(name.clone()).or_default().push(p);
            }
        }
    }
    for files in out.values_mut() {
        files.sort();
    }
    out
}

/// The rule system lint (`engine/lint-rule-system`), when it exists.
///
/// TODO(engine/lint-rule-system): that ticket adds
/// `promptus_shared::rules::lint`; call it here and return its findings
/// as printable lines. Until then the report says the lint is pending.
fn lint_rules(_system: &RuleSystem) -> Option<Vec<String>> {
    None
}

fn scale_name(s: Scale) -> &'static str {
    match s {
        Scale::World => "monde",
        Scale::Place => "lieu",
        Scale::Encounter => "rencontre",
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n > 1 { many } else { one })
}

fn issue_line(i: &Issue) -> String {
    let sev = match i.severity {
        Severity::Error => "erreur",
        Severity::Warning => "avertissement",
    };
    format!("    - [{sev}] {} {} — {}", i.code, i.path, i.detail)
}

fn main() -> ExitCode {
    let content = std::env::args().nth(1).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../content"),
        PathBuf::from,
    );
    let rel = |p: &Path| {
        p.strip_prefix(&content)
            .unwrap_or(p)
            .display()
            .to_string()
    };

    let mut worlds: BTreeMap<String, World> = BTreeMap::new();
    let mut failures = Vec::new();

    for (world, files) in yaml_files(&content.join("rules")) {
        for path in files {
            match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| {
                RuleSystem::from_yaml(&t).map_err(|e| e.to_string())
            }) {
                Ok(s) => worlds.entry(world.clone()).or_default().rules.push((path, s)),
                Err(e) => failures.push(format!("{} : {e}", rel(&path))),
            }
        }
    }
    for (world, files) in yaml_files(&content.join("maps")) {
        for path in files {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| Map::from_yaml(&t).map_err(|e| e.to_string()))
            {
                Ok(m) => worlds.entry(world.clone()).or_default().maps.push((path, m)),
                Err(e) => failures.push(format!("{} : {e}", rel(&path))),
            }
        }
    }
    for (world, files) in yaml_files(&content.join("campaigns")) {
        for path in files {
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| from_yaml(&t).map_err(|e| e.to_string()))
            {
                Ok(c) => worlds
                    .entry(world.clone())
                    .or_default()
                    .campaigns
                    .push((path, c)),
                Err(e) => failures.push(format!("{} : {e}", rel(&path))),
            }
        }
    }

    // A campaign may name any rule system and play on any map.
    let systems: Vec<&RuleSystem> = worlds
        .values()
        .flat_map(|w| w.rules.iter().map(|(_, s)| s))
        .collect();
    let maps: Vec<Map> = worlds
        .values()
        .flat_map(|w| w.maps.iter().map(|(_, m)| m.clone()))
        .collect();

    let mut out = String::new();
    let _ = writeln!(out, "Mondes de Promptus — {}", content.display());
    for (name, w) in &worlds {
        let _ = writeln!(out, "\n== {name} ==");
        for (path, s) in &w.rules {
            let _ = writeln!(
                out,
                "Règles     {} v{} « {} » ({}) : {}, {}, {}",
                s.id,
                s.version,
                s.name,
                rel(path),
                plural(s.classes.len(), "classe", "classes"),
                plural(s.items.len(), "objet", "objets"),
                plural(s.adversaries.len(), "adversaire", "adversaires"),
            );
            match lint_rules(s) {
                Some(lines) if lines.is_empty() => {
                    let _ = writeln!(out, "  contrôle des règles : rien à signaler");
                }
                Some(lines) => {
                    let _ = writeln!(out, "  contrôle des règles :");
                    for l in lines {
                        let _ = writeln!(out, "    - {l}");
                    }
                }
                None => {
                    let _ = writeln!(
                        out,
                        "  contrôle des règles : en attente de engine/lint-rule-system"
                    );
                }
            }
        }
        for (path, m) in &w.maps {
            let _ = writeln!(
                out,
                "Carte      {} « {} » ({}) : {} × {}, échelle {}, {} — valide",
                m.id,
                m.name,
                rel(path),
                m.grid.width(),
                m.grid.height(),
                scale_name(m.scale),
                plural(m.starts.len(), "position de départ", "positions de départ"),
            );
        }
        for (path, c) in &w.campaigns {
            let rules = systems
                .iter()
                .copied()
                .find(|s| s.id == c.rules.id && s.version == c.rules.version);
            let issues = validate_with(
                c,
                &Library {
                    rules,
                    maps: Some(&maps),
                },
            );
            let errors = issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .count();
            let warnings = issues.len() - errors;
            let _ = writeln!(
                out,
                "Campagne   {} « {} » ({}) — règles {} v{}{}",
                c.id,
                c.title,
                rel(path),
                c.rules.id,
                c.rules.version,
                if rules.is_some() { "" } else { " (introuvables)" },
            );
            let _ = writeln!(
                out,
                "  {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                plural(c.acts.len(), "acte", "actes"),
                plural(c.nodes.len(), "scène", "scènes"),
                plural(c.revelations.len(), "révélation", "révélations"),
                plural(c.clues.len(), "indice", "indices"),
                plural(c.party.len(), "personnage", "personnages"),
                plural(c.npcs.len(), "PNJ", "PNJ"),
                plural(c.adversaries.len(), "adversaire", "adversaires"),
                plural(c.locations.len(), "lieu", "lieux"),
                plural(c.items.len(), "objet", "objets"),
                plural(c.factions.len(), "faction", "factions"),
                plural(c.goals.len(), "objectif", "objectifs"),
            );
            let _ = writeln!(
                out,
                "  validateur d'histoire : {}, {}",
                plural(errors, "erreur", "erreurs"),
                plural(warnings, "avertissement", "avertissements"),
            );
            let mut by_code: BTreeMap<&str, usize> = BTreeMap::new();
            for i in &issues {
                *by_code.entry(i.code).or_default() += 1;
            }
            if !by_code.is_empty() {
                let summary: Vec<String> =
                    by_code.iter().map(|(k, n)| format!("{k} ×{n}")).collect();
                let _ = writeln!(out, "  par code : {}", summary.join(", "));
            }
            for i in &issues {
                let _ = writeln!(out, "{}", issue_line(i));
            }
        }
    }
    print!("{out}");

    if failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("\nFichiers qui ne se chargent pas :");
        for f in &failures {
            eprintln!("  - {f}");
        }
        ExitCode::FAILURE
    }
}
