//! The small arithmetic language rule systems write their formulas in:
//! the ability modifier (`(score - 10) / 2`), armour class
//! (`10 + mod(DEX)`), hit points, initiative bonus.
//!
//! Integers only. `/` is **floor** division (rounds towards minus
//! infinity: `(9 - 10) / 2 = -1`), which is what "arrondi vers le bas"
//! means in both witness systems. Functions: `mod(X)` and `score(X)` for
//! an ability, `min(a, b)`, `max(a, b)`. Variables: `score` (inside the
//! modifier formula only) and `level`.

use std::fmt;

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    Num(i64),
    Var(String),
    Mod(String),
    Score(String),
    Neg(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Min(Box<Expr>, Box<Expr>),
    Max(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

/// A parsed formula, kept with its source text for messages and display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    source: String,
    expr: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaError(pub String);

impl fmt::Display for FormulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FormulaError {}

/// What a formula can read while it is evaluated.
pub trait FormulaEnv {
    fn var(&self, name: &str) -> Option<i64>;
    /// The modifier of an ability (`mod(DEX)`).
    fn ability_mod(&self, ability: &str) -> Option<i64>;
    /// The score of an ability (`score(DEX)`).
    fn ability_score(&self, ability: &str) -> Option<i64>;
}

/// Names a formula refers to, so loading can check each one exists.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct References {
    pub vars: Vec<String>,
    pub abilities: Vec<String>,
}

impl Formula {
    pub fn parse(source: &str) -> Result<Self, FormulaError> {
        let tokens = tokenize(source)?;
        let mut p = Parser { tokens, pos: 0 };
        let expr = p.expr()?;
        if p.pos != p.tokens.len() {
            return Err(FormulaError(format!(
                "unexpected `{}` in `{source}`",
                p.tokens[p.pos]
            )));
        }
        Ok(Self {
            source: source.to_string(),
            expr,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn references(&self) -> References {
        let mut refs = References::default();
        collect(&self.expr, &mut refs);
        refs
    }

    pub fn eval(&self, env: &dyn FormulaEnv) -> Result<i64, FormulaError> {
        eval(&self.expr, env)
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.source)
    }
}

impl<'de> Deserialize<'de> for Formula {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Int(i64),
            Text(String),
        }
        let text = match Raw::deserialize(d)? {
            Raw::Int(v) => v.to_string(),
            Raw::Text(t) => t,
        };
        Formula::parse(&text).map_err(serde::de::Error::custom)
    }
}

fn collect(e: &Expr, refs: &mut References) {
    match e {
        Expr::Num(_) => {}
        Expr::Var(v) => refs.vars.push(v.clone()),
        Expr::Mod(a) | Expr::Score(a) => refs.abilities.push(a.clone()),
        Expr::Neg(x) => collect(x, refs),
        Expr::Bin(_, a, b) | Expr::Min(a, b) | Expr::Max(a, b) => {
            collect(a, refs);
            collect(b, refs);
        }
    }
}

fn floor_div(a: i64, b: i64) -> i64 {
    let q = a / b;
    if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

fn eval(e: &Expr, env: &dyn FormulaEnv) -> Result<i64, FormulaError> {
    Ok(match e {
        Expr::Num(n) => *n,
        Expr::Var(v) => env
            .var(v)
            .ok_or_else(|| FormulaError(format!("unknown variable `{v}`")))?,
        Expr::Mod(a) => env
            .ability_mod(a)
            .ok_or_else(|| FormulaError(format!("unknown ability `{a}`")))?,
        Expr::Score(a) => env
            .ability_score(a)
            .ok_or_else(|| FormulaError(format!("unknown ability `{a}`")))?,
        Expr::Neg(x) => -eval(x, env)?,
        Expr::Min(a, b) => eval(a, env)?.min(eval(b, env)?),
        Expr::Max(a, b) => eval(a, env)?.max(eval(b, env)?),
        Expr::Bin(op, a, b) => {
            let (a, b) = (eval(a, env)?, eval(b, env)?);
            match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div => {
                    if b == 0 {
                        return Err(FormulaError("division by zero".into()));
                    }
                    floor_div(a, b)
                }
            }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Num(i64),
    Ident(String),
    Sym(char),
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Num(n) => write!(f, "{n}"),
            Tok::Ident(s) => f.write_str(s),
            Tok::Sym(c) => write!(f, "{c}"),
        }
    }
}

fn tokenize(src: &str) -> Result<Vec<Tok>, FormulaError> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let n = text
                .parse()
                .map_err(|_| FormulaError(format!("number too large in `{src}`")))?;
            out.push(Tok::Num(n));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(chars[start..i].iter().collect()));
        } else if "+-*/(),".contains(c) {
            out.push(Tok::Sym(c));
            i += 1;
        } else {
            return Err(FormulaError(format!(
                "unexpected character `{c}` in `{src}`"
            )));
        }
    }
    if out.is_empty() {
        return Err(FormulaError("empty formula".into()));
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, sym: char) -> bool {
        if self.peek() == Some(&Tok::Sym(sym)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, sym: char) -> Result<(), FormulaError> {
        if self.eat(sym) {
            Ok(())
        } else {
            Err(FormulaError(format!("expected `{sym}`")))
        }
    }

    fn expr(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.term()?;
        loop {
            let op = if self.eat('+') {
                Op::Add
            } else if self.eat('-') {
                Op::Sub
            } else {
                return Ok(left);
            };
            left = Expr::Bin(op, Box::new(left), Box::new(self.term()?));
        }
    }

    fn term(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.unary()?;
        loop {
            let op = if self.eat('*') {
                Op::Mul
            } else if self.eat('/') {
                Op::Div
            } else {
                return Ok(left);
            };
            left = Expr::Bin(op, Box::new(left), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, FormulaError> {
        if self.eat('-') {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Expr, FormulaError> {
        match self.tokens.get(self.pos).cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok(Expr::Num(n))
            }
            Some(Tok::Sym('(')) => {
                self.pos += 1;
                let e = self.expr()?;
                self.expect(')')?;
                Ok(e)
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if !self.eat('(') {
                    return Ok(Expr::Var(name));
                }
                match name.as_str() {
                    "mod" | "score" => {
                        let ability = match self.tokens.get(self.pos).cloned() {
                            Some(Tok::Ident(a)) => a,
                            _ => {
                                return Err(FormulaError(format!(
                                    "`{name}(…)` takes an ability id, as in {name}(DEX)"
                                )));
                            }
                        };
                        self.pos += 1;
                        self.expect(')')?;
                        Ok(if name == "mod" {
                            Expr::Mod(ability)
                        } else {
                            Expr::Score(ability)
                        })
                    }
                    "min" | "max" => {
                        let a = self.expr()?;
                        self.expect(',')?;
                        let b = self.expr()?;
                        self.expect(')')?;
                        Ok(if name == "min" {
                            Expr::Min(Box::new(a), Box::new(b))
                        } else {
                            Expr::Max(Box::new(a), Box::new(b))
                        })
                    }
                    other => Err(FormulaError(format!("unknown function `{other}`"))),
                }
            }
            Some(t) => Err(FormulaError(format!("unexpected `{t}`"))),
            None => Err(FormulaError("formula ends too early".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Env;
    impl FormulaEnv for Env {
        fn var(&self, name: &str) -> Option<i64> {
            match name {
                "score" => Some(9),
                "level" => Some(3),
                _ => None,
            }
        }
        fn ability_mod(&self, a: &str) -> Option<i64> {
            (a == "DEX").then_some(2)
        }
        fn ability_score(&self, a: &str) -> Option<i64> {
            (a == "DEX").then_some(14)
        }
    }

    fn eval(src: &str) -> i64 {
        Formula::parse(src).unwrap().eval(&Env).unwrap()
    }

    #[test]
    fn division_rounds_down_not_towards_zero() {
        // score 9 → -1, as "arrondi vers le bas" requires (truncation gives 0).
        assert_eq!(eval("(score - 10) / 2"), -1);
        assert_eq!(eval("7 / 2"), 3);
        assert_eq!(eval("-7 / 2"), -4);
    }

    #[test]
    fn precedence_parentheses_and_functions() {
        assert_eq!(eval("10 + mod(DEX)"), 12);
        assert_eq!(eval("2 + 3 * 4"), 14);
        assert_eq!(eval("(2 + 3) * 4"), 20);
        assert_eq!(eval("-score + 1"), -8);
        assert_eq!(eval("max(level, score(DEX)) - min(1, 2)"), 13);
    }

    #[test]
    fn reports_what_it_refers_to() {
        let f = Formula::parse("10 + mod(DEX) + level + score(FOR)").unwrap();
        let r = f.references();
        assert_eq!(r.vars, vec!["level"]);
        assert_eq!(r.abilities, vec!["DEX", "FOR"]);
    }

    #[test]
    fn refuses_malformed_formulas() {
        for bad in [
            "", "10 +", "mod(1)", "foo(DEX)", "10 ** 2", "(1 + 2", "1 2", "10 % 3",
        ] {
            assert!(Formula::parse(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn unknown_names_fail_at_evaluation() {
        assert!(Formula::parse("mod(LUCK)").unwrap().eval(&Env).is_err());
        assert!(Formula::parse("speed").unwrap().eval(&Env).is_err());
        assert!(
            Formula::parse("1 / (level - 3)")
                .unwrap()
                .eval(&Env)
                .is_err()
        );
    }
}
