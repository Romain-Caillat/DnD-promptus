//! The rule-system lint (`engine/lint-rule-system`): whether the rules
//! are *good*, where loading only checks they can run. It **reports,
//! never blocks** — a system with issues loads and plays all the same;
//! the GM decides (`docs/lecons-des-parties.md` §3, chain 1).
//!
//! Entry points:
//! - [`lint`] — every check, balance included with [`BalanceParams::default`];
//! - [`lint_with`] — the same for a given campaign length and pace;
//! - [`balance_report`] — the numbers behind the balance checks, for a
//!   report (`Display` prints a plain table).
//!
//! Each [`Issue`] has a stable `code`, a `path` in the style of
//! [`super::RuleError`], an English `detail` and a French `message` for
//! the GM. Codes:
//!
//! | Code | Severity | What |
//! | --- | --- | --- |
//! | `DAMAGE_MODEL_MIXED` | warning | players, NPCs, items or conditions do not share one damage model (fixed vs dice) |
//! | `PRECISION_NOT_APPLIED` | warning | precision values exist but `attack.precision` is `not_applied` |
//! | `PRIMARY_ABILITY_AMBIGUOUS` | warning | `first_primary` with classes that have several primaries |
//! | `ADVERSARY_AC_OFF_TIER` | warning | an NPC's AC differs from its tier's, without `exception` |
//! | `ADVERSARY_AC_FORMULA` | warning | an untiered NPC's AC differs from the AC formula, without `exception` |
//! | `REFERENCE_MISSING` | error | a cited document is not among the `sources` |
//! | `UNDEFINED_TERM` | warning | free text compares with, or rolls, something the system does not define |
//! | `NAME_DUPLICATE` | warning | two items (or actions) share a name |
//! | `COOLDOWN_UNEXPLAINED` | info | cooldowns exist but `cooldowns.note` does not say what they mean |
//! | `TURN_CONTEXTS_DIFFER` | info | a turn context's economy differs from the first one |
//! | `ABILITY_TOTAL_OUTLIER` | warning | a class's ability total differs from the median |
//! | `DAMAGE_PER_TURN_LOW` | info | a damage-dealing class deals less than half the median at level 1 |
//! | `PROGRESSION_MAX_EARLY` | warning | a class reaches the top level before the campaign's last session |

mod balance;
mod text;

use std::collections::{BTreeMap, BTreeSet};

pub use crate::issue::{Issue, Severity};
pub use balance::{
    BalanceParams, BalanceReport, ClassBalance, LevelDamage, SessionXp, Target, balance_report,
};

use super::model::*;
use text::{comparisons, documents_cited, fold, rolls_named, texts};

/// Every check, with the default campaign for the balance ones.
pub fn lint(system: &RuleSystem) -> Vec<Issue> {
    lint_with(system, &BalanceParams::default())
}

/// Every check, the balance ones computed for `params`.
pub fn lint_with(system: &RuleSystem, params: &BalanceParams) -> Vec<Issue> {
    let mut l = Lint {
        s: system,
        issues: Vec::new(),
    };
    l.damage_models();
    l.precision();
    l.primary_ability();
    l.adversary_armor();
    l.references();
    l.free_text_terms();
    l.duplicate_names();
    l.cooldowns();
    l.turn_contexts();
    l.ability_totals();
    let report = balance_report(system, params);
    l.damage_per_turn(&report);
    l.progression(&report);
    l.issues
}

struct Lint<'a> {
    s: &'a RuleSystem,
    issues: Vec<Issue>,
}

impl Lint<'_> {
    fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        path: impl Into<String>,
        detail: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(Issue {
            severity,
            code,
            path: path.into(),
            detail: detail.into(),
            message: Some(message.into()),
        });
    }

    /// One damage model for players, NPCs, items and conditions: the
    /// players' (class actions) is the reference.
    fn damage_models(&mut self) {
        let mut groups: Vec<(&str, &str, Vec<Amount>)> = vec![
            ("classes", "les attaques des joueurs", Vec::new()),
            ("adversaries", "les PNJ", Vec::new()),
            ("items", "les objets", Vec::new()),
            ("conditions", "les états", Vec::new()),
        ];
        fn collect(out: &mut Vec<Amount>, tags: &[Tag], path: &str, owner: &str) {
            for (i, t) in tags.iter().enumerate() {
                match t {
                    Tag::Damage(d) => out.push(Amount {
                        path: format!("{path}[{i}]"),
                        owner: owner.to_string(),
                        amount: d.amount.to_string(),
                        dice: !d.amount.is_flat(),
                    }),
                    Tag::Choice(o) => collect(out, o, &format!("{path}[{i}]"), owner),
                    _ => {}
                }
            }
        }
        for c in &self.s.classes {
            for a in &c.actions {
                let p = format!("classes[{}].actions[{}].tags", c.id, a.id);
                collect(&mut groups[0].2, &a.tags, &p, &a.name);
            }
        }
        for adv in &self.s.adversaries {
            for a in &adv.actions {
                let p = format!("adversaries[{}].actions[{}].tags", adv.id, a.id);
                collect(&mut groups[1].2, &a.tags, &p, &adv.name);
            }
        }
        for it in &self.s.items {
            if let Some(a) = &it.action {
                let p = format!("items[{}].action.tags", it.id);
                collect(&mut groups[2].2, &a.tags, &p, &it.name);
            }
        }
        for c in &self.s.conditions {
            for (i, e) in c.effects.iter().enumerate() {
                if let ConditionEffect::DamagePerTurn(d) = e {
                    groups[3].2.push(Amount {
                        path: format!("conditions[{}].effects[{i}]", c.id),
                        owner: c.name.clone(),
                        amount: d.to_string(),
                        dice: !d.is_flat(),
                    });
                }
            }
        }
        // The reference: the players' model, else the most common one.
        let all = groups.iter().flat_map(|g| g.2.iter());
        let (dice, flat) = all.fold(
            (0, 0),
            |(d, f), a| if a.dice { (d + 1, f) } else { (d, f + 1) },
        );
        if dice == 0 || flat == 0 {
            return;
        }
        let players = &groups[0].2;
        let reference_dice = if players.is_empty() {
            dice > flat
        } else {
            players.iter().filter(|a| a.dice).count() * 2 > players.len()
        };
        let model = |dice: bool| {
            if dice {
                "des dés"
            } else {
                "des dégâts fixes"
            }
        };
        let example = |amounts: &[&Amount]| {
            amounts
                .iter()
                .find(|a| a.dice == reference_dice)
                .map(|a| format!(" (ex. {} : {})", a.owner, a.amount))
                .unwrap_or_default()
        };
        let reference: Vec<&Amount> = groups.iter().flat_map(|g| g.2.iter()).collect();
        let reference_example = example(&reference);
        for (group, who, amounts) in &groups {
            let off: Vec<&Amount> = amounts
                .iter()
                .filter(|a| a.dice != reference_dice)
                .collect();
            let Some(first) = off.first() else { continue };
            let listed = by_owner(&off);
            let reference_who = if players.is_empty() {
                "le reste du système".to_string()
            } else {
                "les attaques des joueurs".to_string()
            };
            self.push(
                Severity::Warning,
                "DAMAGE_MODEL_MIXED",
                first.path.clone(),
                format!(
                    "{group}: {} damage amount(s) use {} where the reference model is {} ({})",
                    off.len(),
                    if first.dice { "dice" } else { "fixed values" },
                    if reference_dice { "dice" } else { "fixed values" },
                    listed,
                ),
                format!(
                    "Deux modèles de dégâts : {who} utilisent {} ({}) alors que {reference_who} utilisent {}{reference_example}. \
                     Un joueur ne sait plus lequel s'applique ; choisissez-en un seul.",
                    model(first.dice),
                    listed,
                    model(reference_dice),
                ),
            );
        }
    }

    /// Precision values that change no roll.
    fn precision(&mut self) {
        if self.s.attack.precision != PrecisionRule::NotApplied {
            return;
        }
        let mut carriers: Vec<String> = Vec::new();
        let mut actions = |name: &str, tags: &[Tag]| {
            let carries = tags.iter().any(|t| match t {
                Tag::Precision(n) => *n != 0,
                Tag::Situational(b) => b.precision != 0,
                Tag::Buff(a) | Tag::Control(a) => a.effects.iter().any(precision_effect),
                _ => false,
            });
            if carries {
                carriers.push(name.to_string());
            }
        };
        for c in &self.s.classes {
            for a in &c.actions {
                actions(&a.name, &a.tags);
            }
        }
        for it in &self.s.items {
            if let Some(a) = &it.action {
                actions(&a.name, &a.tags);
            }
        }
        for adv in &self.s.adversaries {
            for a in &adv.actions {
                actions(&a.name, &a.tags);
            }
        }
        for c in &self.s.conditions {
            if c.effects.iter().any(precision_effect) {
                carriers.push(c.name.clone());
            }
        }
        if carriers.is_empty() {
            return;
        }
        let shown: Vec<&str> = carriers.iter().take(4).map(String::as_str).collect();
        self.push(
            Severity::Warning,
            "PRECISION_NOT_APPLIED",
            "attack.precision",
            format!(
                "{} actions or conditions carry a precision value, but precision is not applied to any roll",
                carriers.len()
            ),
            format!(
                "« Précision » n'est pas définie : {} cartes ou états en donnent ({}…), mais elle ne change aucun jet. \
                 Dites si elle s'ajoute au jet d'attaque, ou retirez-la des cartes.",
                carriers.len(),
                shown.join(", ")
            ),
        );
    }

    /// "Stat principale" when a class has two.
    fn primary_ability(&mut self) {
        if self.s.attack.ability != AttackAbility::FirstPrimary {
            return;
        }
        let ambiguous: Vec<&ClassDef> = self
            .s
            .classes
            .iter()
            .filter(|c| {
                c.primary_abilities.len() > 1
                    && c.actions
                        .iter()
                        .any(|a| a.roll == RollSpec::Attack && a.ability.is_none())
            })
            .collect();
        if ambiguous.is_empty() {
            return;
        }
        let listed: Vec<String> = ambiguous
            .iter()
            .map(|c| format!("{} : {}", c.name, c.primary_abilities.join(" ou ")))
            .collect();
        self.push(
            Severity::Warning,
            "PRIMARY_ABILITY_AMBIGUOUS",
            "attack.ability",
            format!(
                "attacks add the first primary ability, but {} classes have several ({})",
                ambiguous.len(),
                listed.join("; ")
            ),
            format!(
                "« Stat principale » est ambiguë : {} classes en ont plusieurs ({}). \
                 Le moteur prend la première de la liste ; dites laquelle compte, ou prenez la meilleure.",
                ambiguous.len(),
                listed.join(" ; ")
            ),
        );
    }

    /// NPC armour classes against their tier, or against the formula.
    fn adversary_armor(&mut self) {
        let formula = &self.s.stats.armor_class.formula;
        for a in &self.s.adversaries {
            if a.exception.is_some() {
                continue;
            }
            let path = format!("adversaries[{}].armor_class", a.id);
            let tier = a
                .tier
                .as_deref()
                .and_then(|t| self.s.adversary_tiers.iter().find(|x| x.id == t));
            if let Some(t) = tier {
                if a.armor_class != t.armor_class {
                    self.push(
                        Severity::Warning,
                        "ADVERSARY_AC_OFF_TIER",
                        path,
                        format!(
                            "armour class {} where tier `{}` gives {}",
                            a.armor_class, t.id, t.armor_class
                        ),
                        format!(
                            "{} a une CA de {} alors que la table de référence donne {} pour « {} ». \
                             Alignez-la, ou marquez-la comme exception (`exception: <raison>`).",
                            a.name, a.armor_class, t.armor_class, t.name
                        ),
                    );
                }
            } else if let Some(expected) = balance::stat(self.s, &a.abilities, formula, 1)
                && expected != a.armor_class
            {
                self.push(
                    Severity::Warning,
                    "ADVERSARY_AC_FORMULA",
                    path,
                    format!(
                        "armour class {} where `{}` gives {expected}",
                        a.armor_class, formula
                    ),
                    format!(
                        "{} a une CA de {} alors que la formule ({}) donne {expected}. \
                         Corrigez-la, ou marquez-la comme exception (`exception: <raison>`).",
                        a.name, a.armor_class, formula
                    ),
                );
            }
        }
    }

    /// Documents the system sends the reader to must be among its sources.
    fn references(&mut self) {
        let mut reported = BTreeSet::new();
        let mut cited: Vec<(String, String)> = Vec::new();
        for c in &self.s.turn_contexts {
            for (i, r) in c.references.iter().enumerate() {
                cited.push((
                    format!("turn_contexts[{}].references[{i}]", c.id),
                    r.clone(),
                ));
            }
        }
        for t in texts(self.s) {
            for d in documents_cited(&t.text) {
                cited.push((t.path.clone(), d));
            }
        }
        for (path, doc) in cited {
            if self.is_source(&doc) || !reported.insert(basename(&doc).to_string()) {
                continue;
            }
            self.push(
                Severity::Error,
                "REFERENCE_MISSING",
                path,
                format!("`{doc}` is not among the system's sources"),
                format!(
                    "Les règles renvoient à `{doc}`, qui ne fait pas partie des documents du système. \
                     Écrivez-le et ajoutez-le aux sources, ou retirez le renvoi."
                ),
            );
        }
    }

    fn is_source(&self, doc: &str) -> bool {
        self.s.sources.iter().any(|src| {
            src == doc
                || basename(src) == basename(doc)
                || (src.ends_with('/') && doc.starts_with(src.as_str()))
        })
    }

    /// Free text that compares with something undefined, or names a roll
    /// that does not exist.
    fn free_text_terms(&mut self) {
        let mut names: BTreeSet<String> = BTreeSet::new();
        let mut add = |n: &str| {
            names.insert(fold(n.trim()));
        };
        for c in &self.s.classes {
            add(&c.name);
            c.actions.iter().for_each(|a| add(&a.name));
        }
        for it in &self.s.items {
            add(&it.name);
            if let Some(a) = &it.action {
                add(&a.name);
            }
        }
        for a in &self.s.adversaries {
            add(&a.name);
            a.actions.iter().for_each(|x| add(&x.name));
        }
        self.s.conditions.iter().for_each(|c| add(&c.name));
        self.s.resources.iter().for_each(|r| add(&r.name));

        // Rolls that exist: ability checks, attacks, initiative, saves —
        // and heals or damage only when some amount is rolled.
        let mut rolls: BTreeSet<String> =
            ["attaque", "attaques", "initiative", "sauvegarde", "des"]
                .into_iter()
                .map(String::from)
                .collect();
        for a in &self.s.abilities {
            rolls.insert(fold(&a.id));
            rolls.insert(fold(&a.name));
        }
        let (heals_rolled, damage_rolled) = self.rolled_amounts();
        if heals_rolled {
            rolls.extend(["soin", "soins"].map(String::from));
        }
        if damage_rolled {
            rolls.extend(["degat", "degats"].map(String::from));
        }

        let mut seen = BTreeSet::new();
        for t in texts(self.s) {
            for phrase in comparisons(&t.text) {
                let key = fold(&phrase);
                if names.contains(&key) || !seen.insert((t.path.clone(), key)) {
                    continue;
                }
                self.push(
                    Severity::Warning,
                    "UNDEFINED_TERM",
                    t.path.clone(),
                    format!("compares with `{phrase}`, which the system does not define"),
                    format!(
                        "Le texte compare à « {phrase} », que le système ne définit nulle part. \
                         Définissez-le (objet, action…) ou donnez la valeur directement."
                    ),
                );
            }
            for roll in rolls_named(&t.text) {
                let key = fold(&roll);
                if rolls.contains(&key) || !seen.insert((t.path.clone(), key)) {
                    continue;
                }
                self.push(
                    Severity::Warning,
                    "UNDEFINED_TERM",
                    t.path.clone(),
                    format!("names a roll of `{roll}`, but no such roll exists in the system"),
                    format!(
                        "Le texte parle de « jets de {roll} », mais aucun jet de ce genre n'existe dans le système. \
                         Dites ce que l'effet modifie vraiment."
                    ),
                );
            }
        }
    }

    fn rolled_amounts(&self) -> (bool, bool) {
        let (mut heals, mut damage) = (false, false);
        let mut scan = |tags: &[Tag]| {
            for t in tags {
                match t {
                    Tag::Heal(h) if !h.amount.is_flat() => heals = true,
                    Tag::Damage(d) if !d.amount.is_flat() => damage = true,
                    _ => {}
                }
            }
        };
        for c in &self.s.classes {
            c.actions.iter().for_each(|a| scan(&a.tags));
        }
        for it in &self.s.items {
            if let Some(a) = &it.action {
                scan(&a.tags);
            }
        }
        for adv in &self.s.adversaries {
            adv.actions.iter().for_each(|a| scan(&a.tags));
        }
        (heals, damage)
    }

    /// Two items, or two actions, with one name: which one applies?
    fn duplicate_names(&mut self) {
        let mut items: BTreeMap<String, Vec<&ItemDef>> = BTreeMap::new();
        for it in &self.s.items {
            items.entry(fold(it.name.trim())).or_default().push(it);
        }
        for same in items.values().filter(|v| v.len() > 1) {
            let ids: Vec<&str> = same.iter().map(|i| i.id.as_str()).collect();
            let name = &same[0].name;
            self.push(
                Severity::Warning,
                "NAME_DUPLICATE",
                format!("items[{}].name", same[1].id),
                format!("items {} share the name `{name}`", ids.join(", ")),
                format!(
                    "{} objets s'appellent « {name} » ({}). Un joueur ne sait pas lequel il a ; \
                     renommez-en un ou fusionnez-les.",
                    same.len(),
                    ids.join(", ")
                ),
            );
        }
        // Class actions only: an adversary's "Sabre d'abordage" is fine.
        let mut actions: BTreeMap<String, Vec<(String, &ActionDef)>> = BTreeMap::new();
        for c in &self.s.classes {
            for a in &c.actions {
                actions
                    .entry(fold(a.name.trim()))
                    .or_default()
                    .push((format!("classes[{}].actions[{}]", c.id, a.id), a));
            }
        }
        for same in actions.values().filter(|v| v.len() > 1) {
            if same.iter().all(|(_, a)| a.tags == same[0].1.tags) {
                continue;
            }
            let name = &same[0].1.name;
            self.push(
                Severity::Warning,
                "NAME_DUPLICATE",
                format!("{}.name", same[1].0),
                format!("{} class actions named `{name}` differ", same.len()),
                format!(
                    "{} actions de classe s'appellent « {name} » sans faire la même chose ; renommez-en une.",
                    same.len()
                ),
            );
        }
    }

    /// The meaning of "cooldown N" is data; players also need it in words.
    fn cooldowns(&mut self) {
        let any = self
            .s
            .classes
            .iter()
            .flat_map(|c| c.actions.iter())
            .any(|a| a.cooldown() > 0);
        if !any || !self.s.cooldowns.note.trim().is_empty() {
            return;
        }
        let reading = match self.s.cooldowns.meaning {
            CooldownMeaning::SkipNextTurns => {
                "inutilisable pendant vos N prochains tours (recharge 1 = un tour sur deux)"
            }
            CooldownMeaning::TurnOfUseCounts => {
                "le tour d'utilisation compte (recharge 1 = pas deux fois dans le même tour)"
            }
        };
        self.push(
            Severity::Info,
            "COOLDOWN_UNEXPLAINED",
            "cooldowns.note",
            "cooldowns are used but their meaning is not written for the players",
            format!(
                "Les recharges sont lues comme « {reading} », mais aucune phrase ne le dit aux joueurs. \
                 Écrivez-la dans `cooldowns.note`."
            ),
        );
    }

    /// A turn context whose economy differs from the first one: maybe
    /// intended (a ship is not a deck brawl), the GM confirms.
    fn turn_contexts(&mut self) {
        let Some((first, rest)) = self.s.turn_contexts.split_first() else {
            return;
        };
        let limit = |c: &TurnContext, kind: &str| {
            c.limits
                .iter()
                .find(|l| l.kind == kind)
                .map(|l| l.max_per_turn)
        };
        let kind_name = |k: &str| {
            self.s
                .action_kind(k)
                .map(|x| x.name.clone())
                .unwrap_or_else(|| k.to_string())
        };
        let mut found = Vec::new();
        for c in rest {
            if c.actions_per_turn != first.actions_per_turn {
                found.push((
                    format!("turn_contexts[{}].actions_per_turn", c.id),
                    format!(
                        "{} actions per turn in `{}`, {} in `{}`",
                        c.actions_per_turn, c.id, first.actions_per_turn, first.id
                    ),
                    format!(
                        "{} : {} actions par tour, contre {} en « {} ».",
                        c.name, c.actions_per_turn, first.actions_per_turn, first.name
                    ),
                ));
            }
            let kinds: BTreeSet<&str> = c
                .limits
                .iter()
                .chain(first.limits.iter())
                .map(|l| l.kind.as_str())
                .collect();
            for k in kinds {
                let (here, there) = (limit(c, k), limit(first, k));
                if here == there {
                    continue;
                }
                let say = |v: Option<u32>, per_turn: u32| v.unwrap_or(per_turn).to_string();
                let i = c.limits.iter().position(|l| l.kind == k);
                found.push((
                    match i {
                        Some(i) => format!("turn_contexts[{}].limits[{i}]", c.id),
                        None => format!("turn_contexts[{}].limits", c.id),
                    },
                    format!(
                        "at most {} `{k}` per turn in `{}`, {} in `{}`",
                        say(here, c.actions_per_turn),
                        c.id,
                        say(there, first.actions_per_turn),
                        first.id
                    ),
                    format!(
                        "{} : au plus {} « {} » par tour, contre {} en « {} ».",
                        c.name,
                        say(here, c.actions_per_turn),
                        kind_name(k),
                        say(there, first.actions_per_turn),
                        first.name
                    ),
                ));
            }
        }
        for (path, detail, message) in found {
            self.push(
                Severity::Info,
                "TURN_CONTEXTS_DIFFER",
                path,
                detail,
                format!("{message} Confirmez que c'est voulu et dites-le aux joueurs."),
            );
        }
    }

    fn ability_totals(&mut self) {
        let totals: Vec<i32> = self.s.classes.iter().map(balance::ability_total).collect();
        if totals.len() < 3 {
            return;
        }
        let Some(median) = balance::lower_median(totals) else {
            return;
        };
        for c in &self.s.classes {
            let total = balance::ability_total(c);
            if total == median {
                continue;
            }
            self.push(
                Severity::Warning,
                "ABILITY_TOTAL_OUTLIER",
                format!("classes[{}].abilities", c.id),
                format!("ability scores total {total} where the median class has {median}"),
                format!(
                    "{} totalise {total} points de caractéristiques, contre {median} pour les autres classes. \
                     Ajustez un score, ou dites pourquoi.",
                    c.name
                ),
            );
        }
    }

    /// A class that deals damage, but far less than the others, at level 1
    /// against the median target. Support classes (no damage) are not
    /// compared: they play another game.
    fn damage_per_turn(&mut self, report: &BalanceReport) {
        let Some(t) = report
            .targets
            .iter()
            .position(|t| t.armor_class == report.xp_target)
        else {
            return;
        };
        let at_one = |c: &ClassBalance| {
            c.damage_per_turn
                .first()
                .and_then(|d| d.per_target.get(t))
                .copied()
                .unwrap_or(0.0)
        };
        let dealers: Vec<(&ClassBalance, f64)> = report
            .classes
            .iter()
            .map(|c| (c, at_one(c)))
            .filter(|(_, d)| *d > 0.0)
            .collect();
        if dealers.len() < 3 {
            return;
        }
        let mut sorted: Vec<f64> = dealers.iter().map(|(_, d)| *d).collect();
        sorted.sort_by(f64::total_cmp);
        let median = sorted[(sorted.len() - 1) / 2];
        for (c, d) in dealers {
            if d * 2.0 >= median {
                continue;
            }
            self.push(
                Severity::Info,
                "DAMAGE_PER_TURN_LOW",
                format!("classes[{}].actions", c.id),
                format!(
                    "{d:.2} expected damage per turn at level 1 against AC {}, where the median damage dealer does {median:.2}",
                    report.xp_target
                ),
                format!(
                    "{} inflige environ {} dégâts par tour au niveau 1 contre une CA de {}, \
                     moins de la moitié des autres classes offensives ({}). Voulu ?",
                    c.name,
                    french(d),
                    report.xp_target,
                    french(median)
                ),
            );
        }
    }

    fn progression(&mut self, report: &BalanceReport) {
        let params = &report.params;
        let early: Vec<(&ClassBalance, u32)> = report
            .classes
            .iter()
            .filter_map(|c| c.max_level_after_session.map(|n| (c, n)))
            .filter(|(_, n)| *n < params.sessions)
            .collect();
        let Some(&(fastest, session)) = early.iter().min_by_key(|(_, n)| *n) else {
            return;
        };
        let per_session = fastest
            .sessions
            .first()
            .map(|s| s.from_fights + s.from_checks)
            .unwrap_or(0.0);
        let names: Vec<&str> = early.iter().map(|(c, _)| c.name.as_str()).collect();
        self.push(
            Severity::Warning,
            "PROGRESSION_MAX_EARLY",
            "progression.levels",
            format!(
                "{} classes reach level {} before the last of {} sessions ({} after session {session}, ~{per_session:.1} XP in session 1)",
                early.len(),
                self.s.max_level(),
                params.sessions,
                fastest.id,
            ),
            format!(
                "L'XP s'emballe : {} atteignent le niveau {} avant la fin d'une campagne de {} sessions \
                 ({} dès la session {session}, environ {} XP en session 1). \
                 Espacez les paliers, ou donnez moins d'XP par jet réussi.",
                names.join(", "),
                self.s.max_level(),
                params.sessions,
                fastest.name,
                french(per_session),
            ),
        );
    }
}

/// A damage amount and whose it is.
struct Amount {
    path: String,
    owner: String,
    amount: String,
    dice: bool,
}

/// "Gueule-Rouge 1d6+2, Marin 1d4+1 et 7 autres": one amount per owner.
fn by_owner(amounts: &[&Amount]) -> String {
    let mut owners: Vec<(&str, &str)> = Vec::new();
    for a in amounts {
        if !owners.iter().any(|(o, _)| *o == a.owner) {
            owners.push((&a.owner, &a.amount));
        }
    }
    const SHOWN: usize = 4;
    let out: Vec<String> = owners
        .iter()
        .take(SHOWN)
        .map(|(o, a)| format!("{o} {a}"))
        .collect();
    match owners.len().checked_sub(SHOWN) {
        Some(more) if more > 0 => format!("{} et {more} autres", out.join(", ")),
        _ => out.join(", "),
    }
}

fn precision_effect(e: &ConditionEffect) -> bool {
    match e {
        ConditionEffect::Precision(n) | ConditionEffect::PrecisionAgainst(n) => *n != 0,
        ConditionEffect::OnHit(a) => a.effects.iter().any(precision_effect),
        _ => false,
    }
}

/// One decimal, French style: `1,2`.
fn french(v: f64) -> String {
    format!("{v:.1}").replace('.', ",")
}

fn basename(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
}
