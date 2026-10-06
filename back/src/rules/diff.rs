//! A line diff of two versions of a rule system's text, for the GM's
//! comparison screen: what a player reads changed is
//! `rules::changes::rule_changes`; this shows every line, GM-only parts
//! included (adversaries, tiers, notes).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    Same,
    Removed,
    Added,
}

/// One line of the diff. `Same` lines appear only as context around a
/// change; a gap between hunks is a line of kind `Same` with no text
/// and `skipped` set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    /// Unchanged lines left out here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<usize>,
}

/// Lines kept around each change.
const CONTEXT: usize = 2;

/// The lines of `new` against `old`, unchanged runs cut to
/// [`CONTEXT`] lines around each change. Empty when they are equal.
#[must_use]
pub fn lines(old: &str, new: &str) -> Vec<Line> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let full = edit_script(&a, &b);
    if full.iter().all(|(k, _)| *k == LineKind::Same) {
        return Vec::new();
    }
    // Keep a `Same` line only when a change is within CONTEXT lines.
    let near: Vec<bool> = (0..full.len())
        .map(|i| {
            let lo = i.saturating_sub(CONTEXT);
            let hi = (i + CONTEXT).min(full.len() - 1);
            full[lo..=hi].iter().any(|(k, _)| *k != LineKind::Same)
        })
        .collect();
    let mut out = Vec::new();
    let mut skipped = 0;
    for ((kind, text), keep) in full.into_iter().zip(near) {
        if keep {
            if skipped > 0 {
                out.push(Line {
                    kind: LineKind::Same,
                    text: String::new(),
                    skipped: Some(skipped),
                });
                skipped = 0;
            }
            out.push(Line {
                kind,
                text: text.to_string(),
                skipped: None,
            });
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 {
        out.push(Line {
            kind: LineKind::Same,
            text: String::new(),
            skipped: Some(skipped),
        });
    }
    out
}

/// The longest-common-subsequence edit script, removals before
/// additions at each change. Rule systems are a few thousand lines at
/// most, so the quadratic table is small.
fn edit_script<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<(LineKind, &'a str)> {
    // Common prefix and suffix first: most edits touch a few lines.
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (ma, mb) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let (n, m) = (ma.len(), mb.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if ma[i] == mb[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut out: Vec<(LineKind, &str)> = a[..prefix].iter().map(|l| (LineKind::Same, *l)).collect();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && ma[i] == mb[j] {
            out.push((LineKind::Same, ma[i]));
            i += 1;
            j += 1;
        } else if j >= m || (i < n && lcs[i + 1][j] >= lcs[i][j + 1]) {
            out.push((LineKind::Removed, ma[i]));
            i += 1;
        } else {
            out.push((LineKind::Added, mb[j]));
            j += 1;
        }
    }
    out.extend(a[a.len() - suffix..].iter().map(|l| (LineKind::Same, *l)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(lines: &[Line]) -> Vec<(LineKind, &str)> {
        lines.iter().map(|l| (l.kind, l.text.as_str())).collect()
    }

    #[test]
    fn a_changed_value_shows_as_one_line_out_one_line_in_with_its_context() {
        let old = "a\nb\nc\nfacile: 10\nd\ne\nf\ng";
        let new = "a\nb\nc\nfacile: 12\nd\ne\nf\ng";
        let d = lines(old, new);
        assert_eq!(
            kinds(&d),
            [
                (LineKind::Same, ""),
                (LineKind::Same, "b"),
                (LineKind::Same, "c"),
                (LineKind::Removed, "facile: 10"),
                (LineKind::Added, "facile: 12"),
                (LineKind::Same, "d"),
                (LineKind::Same, "e"),
                (LineKind::Same, ""),
            ]
        );
        assert_eq!(d[0].skipped, Some(1));
        assert_eq!(d[7].skipped, Some(2));
    }

    #[test]
    fn equal_texts_have_no_diff_and_insertions_keep_the_rest_aligned() {
        assert!(lines("x\ny", "x\ny").is_empty());
        let d = lines("un\ntrois", "un\ndeux\ntrois");
        assert_eq!(
            kinds(&d),
            [
                (LineKind::Same, "un"),
                (LineKind::Added, "deux"),
                (LineKind::Same, "trois")
            ]
        );
    }
}
