//! Free text of a rule system — descriptions, notes, tactics — and the
//! few things that can be read in it reliably: a comparison with
//! something that has to exist ("par rapport à une épée standard"), a
//! roll that has to exist ("jets de soin"), a document cited by file
//! name ("voir `Combat_Sol.md`").

use super::super::model::*;

/// One piece of free text and where it is.
pub(super) struct Text {
    pub path: String,
    pub text: String,
}

/// Every free-text field of the system, with its path.
pub(super) fn texts(s: &RuleSystem) -> Vec<Text> {
    let mut out = Vec::new();
    let mut push = |path: String, text: &'_ str| {
        if !text.trim().is_empty() {
            out.push((path, text.to_string()));
        }
    };
    push("description".into(), &s.description);
    for a in &s.abilities {
        push(format!("abilities[{}].description", a.id), &a.description);
    }
    for d in &s.difficulties {
        push(
            format!("difficulties[{}].description", d.id),
            &d.description,
        );
    }
    let o = &s.outcomes;
    push(
        "outcomes.critical_failure.description".into(),
        &o.critical_failure.description,
    );
    push(
        "outcomes.failure.description".into(),
        &o.failure.description,
    );
    push(
        "outcomes.success.description".into(),
        &o.success.description,
    );
    push(
        "outcomes.critical_success.description".into(),
        &o.critical_success.description,
    );
    if let Some(g) = &s.group_check {
        push("group_check.note".into(), &g.note);
    }
    push("attack.note".into(), &s.attack.note);
    for k in &s.action_kinds {
        push(
            format!("action_kinds[{}].description", k.id),
            &k.description,
        );
    }
    for c in &s.turn_contexts {
        push(format!("turn_contexts[{}].note", c.id), &c.note);
    }
    push("cooldowns.note".into(), &s.cooldowns.note);
    push("durations.note".into(), &s.durations.note);
    match &s.zero_hp {
        ZeroHpRule::KnockedOut { note, .. } | ZeroHpRule::DeathSaves { note, .. } => {
            push("zero_hp.note".into(), note)
        }
    }
    push("creation.note".into(), &s.creation.note);
    for (i, m) in s.movement.iter().enumerate() {
        push(format!("movement[{i}].note"), &m.note);
    }
    for c in &s.conditions {
        let path = format!("conditions[{}]", c.id);
        push(format!("{path}.description"), &c.description);
        effects(&mut push, &c.effects, &format!("{path}.effects"));
    }
    for c in &s.classes {
        let path = format!("classes[{}]", c.id);
        push(format!("{path}.description"), &c.description);
        for (i, n) in c.notes.iter().enumerate() {
            push(format!("{path}.notes[{i}]"), n);
        }
        for a in &c.actions {
            action(&mut push, a, &format!("{path}.actions[{}]", a.id));
        }
    }
    for it in &s.items {
        let path = format!("items[{}]", it.id);
        push(format!("{path}.description"), &it.description);
        push(format!("{path}.note"), &it.note);
        if let Some(a) = &it.action {
            action(&mut push, a, &format!("{path}.action"));
        }
    }
    for a in &s.adversaries {
        let path = format!("adversaries[{}]", a.id);
        push(format!("{path}.description"), &a.description);
        push(format!("{path}.tactics"), &a.tactics);
        push(format!("{path}.note"), &a.note);
        for act in &a.actions {
            action(&mut push, act, &format!("{path}.actions[{}]", act.id));
        }
    }
    out.into_iter()
        .map(|(path, text)| Text { path, text })
        .collect()
}

fn action(push: &mut impl FnMut(String, &str), a: &ActionDef, path: &str) {
    push(format!("{path}.description"), &a.description);
    tags(push, &a.tags, &format!("{path}.tags"));
}

fn tags(push: &mut impl FnMut(String, &str), tags: &[Tag], path: &str) {
    for (i, t) in tags.iter().enumerate() {
        let p = format!("{path}[{i}]");
        match t {
            Tag::Damage(d) => push(format!("{p}.note"), &d.note),
            Tag::Heal(h) => push(format!("{p}.note"), &h.note),
            Tag::Buff(a) | Tag::Control(a) => apply(push, a, &p),
            Tag::Choice(options) => self::tags(push, options, &p),
            Tag::Note(n) => push(p, n),
            Tag::Precision(_) | Tag::Cooldown(_) | Tag::Situational(_) => {}
        }
    }
}

fn apply(push: &mut impl FnMut(String, &str), a: &ApplySpec, path: &str) {
    push(format!("{path}.note"), &a.note);
    effects(push, &a.effects, &format!("{path}.effects"));
}

fn effects(push: &mut impl FnMut(String, &str), effects: &[ConditionEffect], path: &str) {
    for (i, e) in effects.iter().enumerate() {
        match e {
            ConditionEffect::Narrative(n) => push(format!("{path}[{i}]"), n),
            ConditionEffect::OnHit(a) => apply(push, a, &format!("{path}[{i}]")),
            _ => {}
        }
    }
}

/// Lowercase without French diacritics, one char for one char, so an
/// index in the folded text is an index in the original.
pub(super) fn fold_char(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'à' | 'â' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' => 'i',
        'ô' | 'ö' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        '’' => '\'',
        _ => c,
    }
}

pub(super) fn fold(s: &str) -> String {
    s.chars().map(fold_char).collect()
}

/// What a comparison is made against: "par rapport à une épée standard"
/// → `épée standard` (as written, with its accents).
pub(super) fn comparisons(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let folded: Vec<char> = chars.iter().map(|&c| fold_char(c)).collect();
    let mut out = Vec::new();
    let needle = "par rapport a";
    let mut from = 0;
    while let Some(at) = find_chars(&folded, needle, from) {
        let mut i = at + needle.chars().count();
        from = i;
        // "par rapport au / aux" → the article is glued to "à".
        let rest: String = folded[i..].iter().collect();
        for article in [
            "ux ", "u ", " une ", " un ", " la ", " le ", " les ", " des ", " l'", " ",
        ] {
            if rest.starts_with(article) {
                i += article.chars().count();
                break;
            }
        }
        let start = i;
        while i < chars.len() && !",.;:()!?\"«»`\n".contains(folded[i]) {
            i += 1;
        }
        let phrase: String = chars[start..i].iter().collect();
        let phrase = phrase.trim();
        if !phrase.is_empty() && phrase.split_whitespace().count() <= 5 {
            out.push(phrase.to_string());
        }
    }
    out
}

/// The word after "jet(s) de" / "jet(s) d'": the kind of roll named.
pub(super) fn rolls_named(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let folded: Vec<char> = chars.iter().map(|&c| fold_char(c)).collect();
    let mut out = Vec::new();
    for needle in ["jets de ", "jet de ", "jets d'", "jet d'"] {
        let mut from = 0;
        while let Some(at) = find_chars(&folded, needle, from) {
            from = at + 1;
            // Whole word only: not "rejet de".
            if at > 0 && folded[at - 1].is_alphanumeric() {
                continue;
            }
            let mut i = at + needle.chars().count();
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '-') {
                i += 1;
            }
            if i > start {
                out.push(chars[start..i].iter().collect());
            }
        }
    }
    out
}

/// File names cited in the text (`Combat_Sol.md`, `regles_combat.md`).
pub(super) fn documents_cited(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || "`'\"«»()[],;:".contains(c))
        .map(|w| w.trim_end_matches(['.', '!', '?']))
        .filter(|w| {
            let lower = w.to_ascii_lowercase();
            [".md", ".pdf", ".txt", ".yaml"]
                .iter()
                .any(|ext| lower.ends_with(ext) && lower.len() > ext.len())
        })
        .map(str::to_string)
        .collect()
}

fn find_chars(hay: &[char], needle: &str, from: usize) -> Option<usize> {
    let n: Vec<char> = needle.chars().collect();
    if n.is_empty() || hay.len() < n.len() {
        return None;
    }
    (from..=hay.len() - n.len()).find(|&i| hay[i..i + n.len()] == n[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_a_comparison_is_against() {
        assert_eq!(
            comparisons("+1 dégât par rapport à une épée standard (l'épée…)."),
            vec!["épée standard"]
        );
        assert_eq!(
            comparisons("Deux fois plus loin par rapport au Mousquet court."),
            vec!["Mousquet court"]
        );
        assert!(comparisons("Rien à comparer ici.").is_empty());
    }

    #[test]
    fn reads_the_roll_a_text_names() {
        assert_eq!(
            rolls_named("+1 aux jets de soin, 3 utilisations"),
            vec!["soin"]
        );
        assert_eq!(rolls_named("Jet de FOR pour se libérer."), vec!["FOR"]);
        assert!(rolls_named("Un rejet de la demande.").is_empty());
    }

    #[test]
    fn reads_cited_documents() {
        assert_eq!(
            documents_cited("le combat bascule (voir `Combat_Sol.md`)."),
            vec!["Combat_Sol.md"]
        );
        assert!(documents_cited("Aucun document. Fin.").is_empty());
    }
}
