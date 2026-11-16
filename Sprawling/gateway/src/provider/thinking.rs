// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

pub(crate) mod encoding;

use kernel::event::record::ThinkingStatement;
use kernel::{AxCode, AxError, Effort};

use super::preset::PresetThinking;
use crate::router::AttachedEndpoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffortSet(u8);

impl EffortSet {
    #[must_use]
    pub fn of(levels: &[Effort]) -> EffortSet {
        levels
            .iter()
            .fold(EffortSet::default(), |set, level| set.with(*level))
    }

    #[must_use]
    pub fn with(self, level: Effort) -> EffortSet {
        EffortSet(self.0 | bit(level))
    }

    #[must_use]
    pub fn contains(self, level: Effort) -> bool {
        bit(level) != 0 && self.0 & bit(level) != 0
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Effort> {
        Effort::ALL
            .into_iter()
            .filter(move |level| self.contains(*level))
    }
}

fn bit(level: Effort) -> u8 {
    match level {
        Effort::None => 0,
        Effort::Minimal => 1,
        Effort::Low => 1 << 1,
        Effort::Medium => 1 << 2,
        Effort::High => 1 << 3,
        Effort::XHigh => 1 << 4,
        Effort::Max => 1 << 5,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    Allowed,
    Refused,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferSource {
    Person,
    Upstream,
    Preset,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    Level(Effort),

    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThinkingOffer {
    pub levels: EffortSet,

    pub on: Switch,

    pub default: Option<Effort>,

    pub default_on: Option<bool>,
    pub from: OfferSource,

    pub source: Option<&'static str>,
}

impl ThinkingOffer {
    pub const UNKNOWN: ThinkingOffer = ThinkingOffer {
        levels: EffortSet(0),
        on: Switch::Unknown,
        default: None,
        default_on: None,
        from: OfferSource::Unknown,
        source: None,
    };

    #[must_use]
    pub fn climb(
        upstream: Option<&ThinkingStatement>,
        preset: Option<&'static PresetThinking>,
    ) -> ThinkingOffer {
        match (upstream, preset) {
            (Some(said), _) => ThinkingOffer {
                levels: EffortSet::of(&said.levels),
                on: match said.on {
                    Some(true) => Switch::Allowed,
                    Some(false) => Switch::Refused,
                    None => Switch::Unknown,
                },
                default: said.default,
                default_on: said.default_on,
                from: OfferSource::Upstream,
                source: None,
            },
            (None, Some(row)) => ThinkingOffer {
                levels: EffortSet::of(row.levels),
                on: Switch::Unknown,
                default: row.default,
                default_on: None,
                from: OfferSource::Preset,
                source: Some(row.source),
            },
            (None, None) => ThinkingOffer::UNKNOWN,
        }
    }

    #[must_use]
    pub fn ask(&self, stored: Option<Effort>) -> Option<Ask> {
        match stored {
            Some(level) if self.levels.contains(level) => Some(Ask::Level(level)),
            Some(_) | None => self.fallback(),
        }
    }

    fn fallback(&self) -> Option<Ask> {
        let default = kernel::consts_policy::DEFAULT_EFFORT;
        if self.levels.contains(default) {
            return Some(Ask::Level(default));
        }
        match self.default {
            Some(level) if self.levels.contains(level) => Some(Ask::Level(level)),
            Some(_) | None => {
                (self.levels.is_empty() && self.on == Switch::Allowed).then_some(Ask::On)
            }
        }
    }

    pub fn admit(&self, explicit: Option<Effort>, model: &str) -> Result<(), AxError> {
        let Some(level) = explicit else {
            return Ok(());
        };
        if self.levels.contains(level) {
            return Ok(());
        }
        let offered: Vec<&str> = self.levels.iter().map(Effort::as_str).collect();
        let recovery = if offered.is_empty() {
            format!("{model} offers no thinking level; dispatch without one")
        } else {
            format!(
                "ask {model} for one of the levels it offers ({}), or dispatch without one",
                offered.join(", ")
            )
        };
        Err(AxError::failure(
            AxCode::ConfigInvalid,
            "send a thinking level",
            format!("{model} does not offer {}", level.as_str()),
        )
        .with_recovery(recovery))
    }
}

#[must_use]
pub fn offer_for(endpoint: &AttachedEndpoint, model: &str) -> ThinkingOffer {
    let said = endpoint
        .models
        .iter()
        .find(|row| row.id == model)
        .and_then(|row| row.thinking.as_ref());
    ThinkingOffer::climb(said, super::preset::thinking_for(&endpoint.base_url, model))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn offer(levels: EffortSet, on: Switch, default: Option<Effort>) -> ThinkingOffer {
        ThinkingOffer {
            levels,
            on,
            default,
            ..ThinkingOffer::UNKNOWN
        }
    }

    fn effort() -> impl Strategy<Value = Effort> {
        proptest::sample::select(Effort::ALL.to_vec())
    }

    fn switch() -> impl Strategy<Value = Switch> {
        proptest::sample::select(vec![Switch::Allowed, Switch::Refused, Switch::Unknown])
    }

    proptest! {

        #[test]
        fn the_ladder_keeps_the_lean_properties(
            bits in 0u8..64,
            on in switch(),
            default in proptest::option::of(effort()),
            stored in proptest::option::of(effort()),
            explicit in effort(),
        ) {
            let levels = EffortSet(bits);
            let offer = offer(levels, on, default);
            let sent = offer.ask(stored);
            if let Some(Ask::Level(level)) = sent {
                prop_assert!(levels.contains(level), "{level:?} sent outside {levels:?}");
            }
            if levels.is_empty() && on != Switch::Allowed {
                prop_assert_eq!(sent, None);
            }
            if let Some(level) = stored.filter(|level| levels.contains(*level)) {
                prop_assert_eq!(sent, Some(Ask::Level(level)));
            }
            let admitted = offer.admit(Some(explicit), "m").is_ok();
            prop_assert_eq!(admitted, levels.contains(explicit));
            if admitted {
                prop_assert_eq!(offer.ask(Some(explicit)), Some(Ask::Level(explicit)));
            }
            if sent == Some(Ask::On) {
                prop_assert!(levels.is_empty());
            }
        }
    }

    #[test]
    fn turning_thinking_off_is_never_offered() {
        assert_eq!(EffortSet::of(&[Effort::None]), EffortSet::default());
        assert_eq!(
            EffortSet::of(&[Effort::Max, Effort::None, Effort::Low])
                .iter()
                .collect::<Vec<_>>(),
            vec![Effort::Low, Effort::Max]
        );
    }
}
