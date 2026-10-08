//! Shops and trading, as a player sees them (player/buy-and-trade):
//! built here, by allow-list, like the rest of the projection
//! (`MEMORY.md` §3).
//!
//! Reaches the player: the open shops — name, keeper, the lines on the
//! counter with their name and description (the rules' or the story's
//! player text, or the GM's words), their price now and with the
//! caller's won discount, the stock — the haggling terms (which ability,
//! against what) and the caller's **own** haggle; their purse in the
//! rules' currency; the names of the companions they may give to.
//!
//! Never: a closed shop, a hidden line (« sous le comptoir ») nor how
//! many there are, the NPC id behind a shop, a story item's GM notes or
//! rules effect, another character's haggle, purse or bag.

use promptus_shared::rules::RuleSystem;
use promptus_shared::rules::check::{OutcomeBand, RollBreakdown};
use promptus_shared::rules::trade;
use promptus_shared::story::Campaign;
use serde::Serialize;
use uuid::Uuid;

use super::PlayView;
use crate::shops::{HaggleResult, Shop};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeView {
    /// The caller's purse in the rules' currency; `None` for a spectator
    /// or rules without one.
    pub purse: Option<PurseView>,
    pub shops: Vec<ShopView>,
    /// The other characters in play at the table, to give to.
    pub companions: Vec<CompanionView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurseView {
    pub name: String,
    pub abbr: String,
    pub amount: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionView {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopView {
    pub id: Uuid,
    pub name: String,
    pub keeper: String,
    /// The currency's short name (« PO »).
    pub abbr: String,
    pub lines: Vec<LineView>,
    pub haggle: Option<HaggleTermsView>,
    /// What the keeper adds to every price since a botched haggle.
    pub surcharge: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineView {
    pub key: String,
    pub name: String,
    pub description: String,
    pub price: u32,
    /// The price with the caller's won discount, while they have it.
    pub discounted_price: Option<u32>,
    /// Units left; `None` for as many as wanted.
    pub stock: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HaggleTermsView {
    pub ability: String,
    pub ability_name: String,
    pub difficulty: i32,
    /// The difficulty's name in the rules (« Moyen »), when it has one.
    pub label: Option<String>,
    pub discount_percent: u32,
    /// The caller's own haggle here, once rolled.
    pub mine: Option<MyHaggleView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyHaggleView {
    pub band: OutcomeBand,
    /// The outcome's name in the rules (« Réussite »).
    pub outcome: String,
    pub roll: RollBreakdown,
    pub discount_left: bool,
}

/// Everything the trade API read.
pub struct TradeInput<'a> {
    pub rules: &'a RuleSystem,
    pub story: &'a Campaign,
    pub shops: &'a [Shop],
    /// The caller's character and their haggles; `None` for a spectator.
    pub mine: Option<(Uuid, &'a PlayView)>,
    pub haggles: &'a [HaggleResult],
    /// Every character in play: id and name.
    pub table: &'a [(Uuid, String)],
}

/// Trading as the caller may see it.
#[must_use]
pub fn project_trade(input: &TradeInput<'_>) -> TradeView {
    let rules = input.rules;
    let currency = rules.currency();
    let me = input.mine.map(|(id, _)| id);
    let shops = input
        .shops
        .iter()
        .filter(|s| s.open)
        .map(|s| {
            let mine = me.and_then(|c| {
                input
                    .haggles
                    .iter()
                    .find(|h| h.shop_id == s.id && h.character_id == c)
            });
            let discount = s
                .haggle
                .as_ref()
                .filter(|_| mine.is_some_and(|h| h.discount_left))
                .map(|h| h.discount_percent);
            ShopView {
                id: s.id,
                name: s.name.clone(),
                keeper: s.keeper.clone(),
                abbr: rules
                    .resources
                    .iter()
                    .find(|r| r.id == s.currency)
                    .map_or_else(String::new, |r| r.abbr.clone()),
                lines: s
                    .lines
                    .iter()
                    .filter(|l| !l.hidden)
                    .map(|l| {
                        let (name, description) = l.display(rules, input.story);
                        LineView {
                            key: l.key.clone(),
                            name,
                            description,
                            price: trade::price(l.price, s.surcharge, None),
                            discounted_price: discount
                                .map(|d| trade::price(l.price, s.surcharge, Some(d))),
                            stock: l.stock,
                        }
                    })
                    .collect(),
                haggle: s.haggle.as_ref().map(|h| HaggleTermsView {
                    ability: h.ability.clone(),
                    ability_name: rules
                        .ability(&h.ability)
                        .map_or_else(|| h.ability.clone(), |a| a.name.clone()),
                    difficulty: h.difficulty,
                    label: rules
                        .difficulties
                        .iter()
                        .find(|d| d.value == h.difficulty)
                        .map(|d| d.name.clone()),
                    discount_percent: h.discount_percent,
                    mine: mine.map(|m| MyHaggleView {
                        band: m.band,
                        outcome: m.band.name(rules).to_string(),
                        roll: m.roll.clone(),
                        discount_left: m.discount_left,
                    }),
                }),
                surcharge: s.surcharge,
            }
        })
        .collect();
    TradeView {
        purse: match (input.mine, currency) {
            (Some((_, play)), Some(c)) => {
                play.resources
                    .iter()
                    .find(|r| r.id == c.id)
                    .map(|r| PurseView {
                        name: r.name.clone(),
                        abbr: r.abbr.clone(),
                        amount: r.amount,
                    })
            }
            _ => None,
        },
        shops,
        companions: input
            .table
            .iter()
            .filter(|(id, _)| Some(*id) != me)
            .map(|(id, name)| CompanionView {
                id: *id,
                name: name.clone(),
            })
            .collect(),
    }
}
