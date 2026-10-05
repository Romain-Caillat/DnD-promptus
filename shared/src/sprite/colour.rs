//! Colours of the pixel sprites, and the three-tone shading.
//!
//! Shading lightens or darkens a colour in HLS space exactly as the
//! design prototypes do (`docs/design/sprite-prototype.py`, Python's
//! `colorsys` and `round`), so the app and the boards agree to the bit.

use std::fmt;

/// An opaque sRGB colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Parse `#RRGGBB` (case-insensitive). Anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Self(byte(0)?, byte(2)?, byte(4)?))
    }

    /// The prototype's `adj(colour, f)`: below 1 scales lightness down,
    /// above 1 moves it towards white by `f - 1`.
    pub fn adjust(self, factor: f64) -> Self {
        let (h, l, s) = rgb_to_hls(
            f64::from(self.0) / 255.0,
            f64::from(self.1) / 255.0,
            f64::from(self.2) / 255.0,
        );
        let l = if factor < 1.0 {
            l * factor
        } else {
            l + (1.0 - l) * (factor - 1.0)
        }
        .clamp(0.0, 1.0);
        let (r, g, b) = hls_to_rgb(h, l, s);
        Self(channel(r), channel(g), channel(b))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

/// Lit face: the top edge of a shape.
pub const LIGHT: f64 = 1.28;
/// Shadowed face: the back and bottom edges of a shape.
pub const DARK: f64 = 0.72;

/// Python's `round(x * 255)`: halves go to the even neighbour.
fn channel(x: f64) -> u8 {
    // In range by construction: x is in [0, 1].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let v = (x * 255.0).round_ties_even() as u8;
    v
}

// `colorsys.rgb_to_hls` and `colorsys.hls_to_rgb`, line for line.

fn rgb_to_hls(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let maxc = r.max(g).max(b);
    let minc = r.min(g).min(b);
    let sumc = maxc + minc;
    let rangec = maxc - minc;
    let l = sumc / 2.0;
    #[allow(clippy::float_cmp)]
    if minc == maxc {
        return (0.0, l, 0.0);
    }
    let s = if l <= 0.5 {
        rangec / sumc
    } else {
        rangec / (2.0 - maxc - minc)
    };
    let rc = (maxc - r) / rangec;
    let gc = (maxc - g) / rangec;
    let bc = (maxc - b) / rangec;
    #[allow(clippy::float_cmp)]
    let h = if r == maxc {
        bc - gc
    } else if g == maxc {
        2.0 + rc - bc
    } else {
        4.0 + gc - rc
    };
    ((h / 6.0).rem_euclid(1.0), l, s)
}

fn hls_to_rgb(h: f64, l: f64, s: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        return (l, l, l);
    }
    let m2 = if l <= 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let m1 = 2.0 * l - m2;
    (
        hue(m1, m2, h + 1.0 / 3.0),
        hue(m1, m2, h),
        hue(m1, m2, h - 1.0 / 3.0),
    )
}

fn hue(m1: f64, m2: f64, h: f64) -> f64 {
    let h = h.rem_euclid(1.0);
    if h < 1.0 / 6.0 {
        m1 + (m2 - m1) * h * 6.0
    } else if h < 0.5 {
        m2
    } else if h < 2.0 / 3.0 {
        m1 + (m2 - m1) * (2.0 / 3.0 - h) * 6.0
    } else {
        m1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_refuses_anything_else() {
        assert_eq!(Rgb::parse("#4f6D9a"), Some(Rgb(0x4F, 0x6D, 0x9A)));
        assert_eq!(Rgb::parse("4F6D9A"), None);
        assert_eq!(Rgb::parse("#4F6D9"), None);
        assert_eq!(Rgb::parse("#4F6D9G"), None);
    }

    #[test]
    fn shades_like_the_prototype() {
        // Values printed by docs/design/avatar.py (`css_vars`).
        let skin = Rgb::parse("#E8A982").unwrap();
        assert_eq!(skin.adjust(LIGHT).to_string(), "#EEC1A5");
        assert_eq!(skin.adjust(DARK).to_string(), "#D86E2C");
        let outfit = Rgb::parse("#4F6D9A").unwrap();
        assert_eq!(outfit.adjust(LIGHT).to_string(), "#7B95BB");
        assert_eq!(outfit.adjust(DARK).to_string(), "#394E6F");
    }
}
