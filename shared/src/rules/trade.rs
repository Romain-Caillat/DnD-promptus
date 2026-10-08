//! Buying from a shop (player/buy-and-trade): what a line costs once the
//! keeper's mood (a surcharge after a botched haggle) and a won haggle
//! (a discount on one purchase) are counted. Prices are whole units of
//! the system's currency, and every rounding goes the merchant's way,
//! as Dents-de-Fer does (« arrondi en faveur de Dents-de-Fer »).

use serde::{Deserialize, Serialize};

use super::check::OutcomeBand;

/// What a haggle won, by the band of its roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaggleOutcome {
    /// One purchase at a discount, of the player's choice.
    pub discount: bool,
    /// Every price of the shop rises by this much, for everyone.
    pub surcharge: u32,
    /// What the keeper hides under the counter comes out.
    pub reveals: bool,
}

/// The four outcomes of a haggle, as the shop's terms set them: a
/// success wins a discount, a natural 20 also brings out the hidden
/// stock when `critical_reveals`, a natural 1 adds `fumble_surcharge`
/// to every price, a failure changes nothing.
#[must_use]
pub fn haggle_outcome(
    band: OutcomeBand,
    fumble_surcharge: u32,
    critical_reveals: bool,
) -> HaggleOutcome {
    match band {
        OutcomeBand::CriticalFailure => HaggleOutcome {
            discount: false,
            surcharge: fumble_surcharge,
            reveals: false,
        },
        OutcomeBand::Failure => HaggleOutcome {
            discount: false,
            surcharge: 0,
            reveals: false,
        },
        OutcomeBand::Success => HaggleOutcome {
            discount: true,
            surcharge: 0,
            reveals: false,
        },
        OutcomeBand::CriticalSuccess => HaggleOutcome {
            discount: true,
            surcharge: 0,
            reveals: critical_reveals,
        },
    }
}

/// The price of one unit: the line's price plus the shop's surcharge,
/// less `discount_percent` (0 to 100) when a won haggle is spent on it,
/// rounded up.
#[must_use]
pub fn price(base: u32, surcharge: u32, discount_percent: Option<u32>) -> u32 {
    let full = base.saturating_add(surcharge);
    match discount_percent {
        Some(d) => {
            let keep = 100 - d.min(100);
            // Ceiling of full × keep / 100, in u64 to never overflow.
            u32::try_from((u64::from(full) * u64::from(keep)).div_ceil(100)).unwrap_or(u32::MAX)
        }
        None => full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kerjean_halves_one_price_rounded_his_way() {
        // Acte 1, scène 3: « un objet au choix à moitié prix (arrondi en
        // faveur de Dents-de-Fer) ». The sword is 15: 8, not 7.
        assert_eq!(price(15, 0, Some(50)), 8);
        assert_eq!(price(20, 0, Some(50)), 10);
        assert_eq!(price(15, 0, None), 15);
    }

    #[test]
    fn a_botched_haggle_raises_every_price_before_any_discount() {
        // « Il augmente tous ses prix de 2 PO pour le dérangement. »
        let botched = haggle_outcome(OutcomeBand::CriticalFailure, 2, true);
        assert_eq!(botched.surcharge, 2);
        assert!(!botched.discount);
        assert_eq!(price(12, botched.surcharge, None), 14);
        assert_eq!(price(12, 2, Some(50)), 7);
    }

    #[test]
    fn only_a_natural_twenty_brings_out_the_hidden_stock() {
        let won = haggle_outcome(OutcomeBand::Success, 2, true);
        let crit = haggle_outcome(OutcomeBand::CriticalSuccess, 2, true);
        assert!(won.discount && !won.reveals);
        assert!(crit.discount && crit.reveals);
        assert!(!haggle_outcome(OutcomeBand::CriticalSuccess, 2, false).reveals);
        assert_eq!(
            haggle_outcome(OutcomeBand::Failure, 2, true),
            HaggleOutcome {
                discount: false,
                surcharge: 0,
                reveals: false
            }
        );
    }

    #[test]
    fn a_full_discount_is_free_and_more_is_no_less() {
        assert_eq!(price(9, 0, Some(100)), 0);
        assert_eq!(price(9, 0, Some(150)), 0);
        assert_eq!(price(u32::MAX, 5, None), u32::MAX);
    }
}
