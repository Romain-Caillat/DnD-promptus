//! Dice expressions (`NdX+M`, or a flat number) and the injectable source
//! of randomness every roll goes through.
//!
//! The server owns the dice: it rolls with [`SeededDice`] seeded from the
//! OS, and tests script exact faces with [`ScriptedDice`] or replay a seed.

use std::collections::VecDeque;
use std::fmt;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Where dice faces come from. One call is one die.
pub trait DiceSource {
    /// A face in `1..=faces`.
    fn roll(&mut self, faces: u32) -> u32;
}

/// A reproducible stream of faces: the same seed always gives the same
/// rolls, on every platform and rand release (ChaCha8 is specified).
pub struct SeededDice(ChaCha8Rng);

impl SeededDice {
    pub fn new(seed: u64) -> Self {
        Self(ChaCha8Rng::seed_from_u64(seed))
    }

    /// Seeded from the operating system: what the server uses at the table.
    pub fn from_os() -> Self {
        Self(ChaCha8Rng::from_os_rng())
    }
}

impl DiceSource for SeededDice {
    fn roll(&mut self, faces: u32) -> u32 {
        self.0.random_range(1..=faces)
    }
}

/// Faces decided in advance, consumed in order — for tests that need to
/// say "the d20 shows 14". Panics when it runs out or when a scripted
/// face cannot exist on the die rolled: both are bugs in the test.
pub struct ScriptedDice(VecDeque<u32>);

impl ScriptedDice {
    pub fn new(faces: impl IntoIterator<Item = u32>) -> Self {
        Self(faces.into_iter().collect())
    }

    /// Faces not consumed yet.
    pub fn remaining(&self) -> usize {
        self.0.len()
    }
}

impl DiceSource for ScriptedDice {
    fn roll(&mut self, faces: u32) -> u32 {
        let face = self
            .0
            .pop_front()
            .unwrap_or_else(|| panic!("scripted dice exhausted (a d{faces} was rolled)"));
        assert!(
            (1..=faces).contains(&face),
            "scripted face {face} cannot come up on a d{faces}"
        );
        face
    }
}

/// Upper bounds that keep a typo (`100d6`, `1d2000`) from becoming a rule.
pub const MAX_DICE: u32 = 100;
pub const MAX_FACES: u32 = 1000;

/// `count` dice of `faces` faces plus `modifier`; `count == 0` is a flat
/// number (`"3"` is fixed damage, `"1d6+2"` is rolled damage — both are
/// the same type so a rule system has one damage model in code, whatever
/// its data says).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiceExpr {
    pub count: u32,
    pub faces: u32,
    pub modifier: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceParseError {
    pub input: String,
    pub reason: &'static str,
}

impl fmt::Display for DiceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid dice expression `{}`: {}",
            self.input, self.reason
        )
    }
}

impl std::error::Error for DiceParseError {}

impl DiceExpr {
    pub const fn flat(value: i32) -> Self {
        Self {
            count: 0,
            faces: 0,
            modifier: value,
        }
    }

    pub const fn is_flat(&self) -> bool {
        self.count == 0
    }

    /// Parses `"3"`, `"-1"`, `"1d6"`, `"2d6+3"`, `"1d8 - 1"` (spaces and
    /// case are ignored). `"d20"`, `"1d20+3+5"` and empty input are refused.
    pub fn parse(input: &str) -> Result<Self, DiceParseError> {
        let err = |reason| DiceParseError {
            input: input.to_string(),
            reason,
        };
        let body: String = input
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();
        if body.is_empty() {
            return Err(err("empty"));
        }
        if let Ok(flat) = body.parse::<i32>() {
            return Ok(Self::flat(flat));
        }
        let (count, rest) = body
            .split_once('d')
            .ok_or_else(|| err("expected NdX+M or a number"))?;
        if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
            return Err(err("the number of dice must be written, as in 1d20"));
        }
        let (faces, modifier) = match rest.find(['+', '-']) {
            Some(at) => {
                let (faces, signed) = rest.split_at(at);
                let digits = &signed[1..];
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(err("the modifier must be a single number"));
                }
                let value: i32 = digits.parse().map_err(|_| err("modifier too large"))?;
                (
                    faces,
                    if signed.starts_with('-') {
                        -value
                    } else {
                        value
                    },
                )
            }
            None => (rest, 0),
        };
        if faces.is_empty() || !faces.bytes().all(|b| b.is_ascii_digit()) {
            return Err(err("the number of faces must be a number"));
        }
        let count: u32 = count.parse().map_err(|_| err("too many dice"))?;
        let faces: u32 = faces.parse().map_err(|_| err("too many faces"))?;
        if count == 0 || count > MAX_DICE {
            return Err(err("between 1 and 100 dice"));
        }
        if faces == 0 || faces > MAX_FACES {
            return Err(err("between 1 and 1000 faces"));
        }
        Ok(Self {
            count,
            faces,
            modifier,
        })
    }

    /// Smallest and largest total this expression can produce.
    pub fn range(&self) -> (i32, i32) {
        let n = self.count as i32;
        (n + self.modifier, n * self.faces as i32 + self.modifier)
    }

    pub fn roll(&self, dice: &mut dyn DiceSource) -> DiceRoll {
        let faces: Vec<u32> = (0..self.count).map(|_| dice.roll(self.faces)).collect();
        let total = faces.iter().map(|&f| f as i32).sum::<i32>() + self.modifier;
        DiceRoll {
            expr: *self,
            faces,
            total,
        }
    }
}

impl fmt::Display for DiceExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_flat() {
            return write!(f, "{}", self.modifier);
        }
        write!(f, "{}d{}", self.count, self.faces)?;
        match self.modifier {
            0 => Ok(()),
            m if m > 0 => write!(f, "+{m}"),
            m => write!(f, "{m}"),
        }
    }
}

impl Serialize for DiceExpr {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DiceExpr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // `amount: 3` and `amount: "1d6+2"` are both natural to write.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Int(i32),
            Text(String),
        }
        match Raw::deserialize(d)? {
            Raw::Int(v) => Ok(Self::flat(v)),
            Raw::Text(t) => Self::parse(&t).map_err(serde::de::Error::custom),
        }
    }
}

/// One rolled expression: every face, and the total with the modifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceRoll {
    pub expr: DiceExpr,
    pub faces: Vec<u32>,
    pub total: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_forms_rule_files_use() {
        let d = |count, faces, modifier| DiceExpr {
            count,
            faces,
            modifier,
        };
        assert_eq!(DiceExpr::parse("1d20").unwrap(), d(1, 20, 0));
        assert_eq!(DiceExpr::parse("2d6+3").unwrap(), d(2, 6, 3));
        assert_eq!(DiceExpr::parse("1d8 - 1").unwrap(), d(1, 8, -1));
        assert_eq!(DiceExpr::parse("1D4+2").unwrap(), d(1, 4, 2));
        assert_eq!(DiceExpr::parse("5").unwrap(), DiceExpr::flat(5));
        assert_eq!(DiceExpr::parse("-1").unwrap(), DiceExpr::flat(-1));
    }

    #[test]
    fn refuses_garbage_and_out_of_range_dice() {
        for bad in [
            "", "XYZ", "d20", "1d20+3+5", "1d", "1d20+", "0d20", "200d6", "1d0", "1d2000", "1d6*2",
        ] {
            assert!(DiceExpr::parse(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn display_round_trips() {
        for text in ["1d20", "2d6+3", "1d8-1", "4", "-2"] {
            assert_eq!(DiceExpr::parse(text).unwrap().to_string(), text);
        }
    }

    #[test]
    fn rolls_every_die_and_adds_the_modifier_once() {
        let roll = DiceExpr::parse("3d6+2")
            .unwrap()
            .roll(&mut ScriptedDice::new([1, 4, 6]));
        assert_eq!(roll.faces, vec![1, 4, 6]);
        assert_eq!(roll.total, 13);
    }

    #[test]
    fn a_flat_amount_rolls_nothing() {
        let mut dice = ScriptedDice::new([]);
        let roll = DiceExpr::flat(7).roll(&mut dice);
        assert_eq!(roll.total, 7);
        assert!(roll.faces.is_empty());
    }

    #[test]
    fn a_seed_replays_the_same_rolls_and_stays_on_the_die() {
        let expr = DiceExpr::parse("10d20").unwrap();
        let a = expr.roll(&mut SeededDice::new(42));
        let b = expr.roll(&mut SeededDice::new(42));
        assert_eq!(a, b);
        assert!(a.faces.iter().all(|f| (1..=20).contains(f)));
        let c = expr.roll(&mut SeededDice::new(43));
        assert_ne!(a.faces, c.faces);
    }

    #[test]
    fn seeded_faces_cover_the_whole_die() {
        let mut dice = SeededDice::new(7);
        let mut seen = [false; 6];
        for _ in 0..200 {
            seen[dice.roll(6) as usize - 1] = true;
        }
        assert!(
            seen.iter().all(|s| *s),
            "every face of a d6 comes up in 200 rolls"
        );
    }
}
