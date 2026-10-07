//! The open shop as a player sees it (player/buy-and-trade): its name,
//! the lines on the counter with the caller's own price (their haggle
//! counted), the stock, whether they already haggled each line, how
//! the shop lets them haggle, and their purse.
//!
//! Never: a line kept under the counter, another player's haggle, an
//! item's GM note (`ItemDef::note`).

use promptus_shared::rules::RuleSystem;
use serde::Serialize;
use uuid::Uuid;

use crate::shops::Shop;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopView {
    pub name: String,
    pub lines: Vec<ShopLineView>,
    pub haggle: Option<HaggleView>,
    /// The caller's coins; `None` for a spectator or a character not in
    /// play.
    pub purse: Option<PurseView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopLineView {
    pub id: String,
    pub name: String,
    pub description: String,
    /// What it costs the caller.
    pub price: u32,
    /// The GM's price, before the caller's discount.
    pub list_price: u32,
    pub stock: Option<u32>,
    /// `Some(won)` once the caller haggled this line.
    pub haggled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HaggleView {
    pub ability: String,
    pub ability_name: String,
    pub difficulty: i32,
    pub discount: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurseView {
    pub name: String,
    pub abbr: String,
    pub amount: i32,
}

/// The open `shop` as `player` sees it, with `coins` in their purse.
#[must_use]
pub fn project_shop(shop: &Shop, rules: &RuleSystem, player: Uuid, coins: Option<i32>) -> ShopView {
    ShopView {
        name: shop.name.clone(),
        lines: shop
            .lines
            .iter()
            .filter(|l| !l.hidden)
            .map(|l| ShopLineView {
                id: l.id.clone(),
                name: l.display_name(rules),
                description: l.display_description(rules),
                price: shop.price_for(l, player),
                list_price: l.price,
                stock: l.stock,
                haggled: shop.haggle_of(&l.id, player).map(|h| h.success),
            })
            .collect(),
        haggle: shop.haggle.as_ref().map(|h| HaggleView {
            ability: h.ability.clone(),
            ability_name: rules
                .abilities
                .iter()
                .find(|a| a.id == h.ability)
                .map_or_else(|| h.ability.clone(), |a| a.name.clone()),
            difficulty: h.difficulty,
            discount: h.discount,
        }),
        purse: rules
            .resources
            .first()
            .zip(coins)
            .map(|(r, amount)| PurseView {
                name: r.name.clone(),
                abbr: r.abbr.clone(),
                amount,
            }),
    }
}
