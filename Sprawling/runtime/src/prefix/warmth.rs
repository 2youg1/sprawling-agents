// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use kernel::keep_warm::{CacheUse, KeepWarm, renewal_due};
use kernel::{
    AxError, B3Hash, Ceiling, EarlyCalls, Increments, Model, ModelRequest, ModelReturn, TimeMs,
};

#[derive(Debug, Clone)]
pub struct Warmth {
    setting: KeepWarm,
    kept: BTreeMap<[B3Hash; 4], Kept>,
}

#[derive(Debug, Clone)]
struct Kept {
    request: ModelRequest<'static>,
    cache: CacheUse,
}

impl Warmth {
    pub fn new(setting: KeepWarm) -> Warmth {
        Warmth {
            setting,
            kept: BTreeMap::new(),
        }
    }

    pub fn sent(&mut self, request: &ModelRequest, at_ms: u64) {
        match self.setting {
            KeepWarm::Off => return,
            KeepWarm::FiveMinute => {}
        }
        self.kept.insert(
            request.segments,
            Kept {
                request: request.clone().into_owned(),
                cache: CacheUse::sent(at_ms),
            },
        );
    }

    pub fn next_due(&self, lead: &Lead) -> Option<u64> {
        self.kept
            .values()
            .filter_map(|kept| renewal_due(self.setting, kept.cache, lead.ms()))
            .min()
    }

    pub fn renew_due(
        &mut self,
        lead: &Lead,
        now_ms: u64,
        mut send: impl FnMut(&ModelRequest) -> Result<ModelReturn, AxError>,
    ) -> RenewalRound {
        let (setting, lead_ms) = (self.setting, lead.ms());
        self.kept
            .retain(|_, kept| renewal_due(setting, kept.cache, lead_ms).is_some());
        let mut round = RenewalRound::default();
        for kept in self.kept.values_mut() {
            if renewal_due(setting, kept.cache, lead_ms).is_some_and(|due| due <= now_ms) {
                match send(&renewal_of(&kept.request)) {
                    Ok(answer) => {
                        round.answers.push(answer);
                        kept.cache = kept.cache.renewed();
                    }
                    Err(refused) => {
                        round.refused = Some(refused);
                        break;
                    }
                }
            }
        }
        round
    }
}

#[derive(Debug, Default)]
pub struct RenewalRound {
    pub answers: Vec<ModelReturn>,

    pub refused: Option<AxError>,
}

fn renewal_of(request: &ModelRequest<'static>) -> ModelRequest<'static> {
    let mut renewal = request.clone();
    renewal.chat.max_tokens = Ceiling::new(1);
    renewal
}

#[derive(Debug, Clone, Default)]
pub struct Lead(Arc<AtomicU64>);

impl Lead {
    pub fn ms(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    fn measured(&self, ms: u64) {
        self.0.store(ms, Ordering::Relaxed);
    }
}

pub struct Warmed<C> {
    model: Box<dyn Model + Send>,
    warmth: Warmth,
    clock: C,
    lead: Lead,
}

impl<C: FnMut() -> Result<TimeMs, AxError>> Warmed<C> {
    pub fn new(model: Box<dyn Model + Send>, setting: KeepWarm, clock: C, lead: Lead) -> Warmed<C> {
        Warmed {
            model,
            warmth: Warmth::new(setting),
            clock,
            lead,
        }
    }

    pub fn next_due(&self) -> Option<u64> {
        self.warmth.next_due(&self.lead)
    }

    pub fn renew_due(&mut self, now_ms: u64) -> RenewalRound {
        let Warmed {
            model,
            warmth,
            clock,
            lead,
        } = self;
        let lead: &Lead = lead;
        warmth.renew_due(lead, now_ms, |renewal| {
            let left = clock()?;
            let answer = model.call(renewal)?;
            let back = clock()?;
            lead.measured(back.value().saturating_sub(left.value()));
            Ok(answer)
        })
    }

    fn recorded(
        &mut self,
        request: &ModelRequest,
        send: impl FnOnce(&mut dyn Model, Increments<'_>) -> Result<ModelReturn, AxError>,
    ) -> Result<ModelReturn, AxError> {
        let Warmed {
            model,
            warmth,
            clock,
            lead,
        } = self;
        let left = clock()?;
        let mut first = None;
        let answer = send(model.as_mut(), &mut |_| {
            if first.is_none() {
                first = Some(clock());
            }
        })?;
        if let Some(first) = first {
            lead.measured(first?.value().saturating_sub(left.value()));
        }
        warmth.sent(request, left.value());
        Ok(answer)
    }
}

impl<C: FnMut() -> Result<TimeMs, AxError>> Model for Warmed<C> {
    fn provider_account(&self) -> Option<kernel::event::record::ProviderAccountBinding> {
        self.model.provider_account()
    }

    fn account_roster(&self) -> Option<kernel::model::AccountRoster> {
        self.model.account_roster()
    }

    fn select_account(&mut self, account: &kernel::ServerLabel) {
        self.model.select_account(account);
    }

    fn prefix_held(&self) -> Option<kernel::event::record::PrefixHeld> {
        self.model.prefix_held()
    }

    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.recorded(req, |model, _| model.call(req))
    }

    fn call_streaming(
        &mut self,
        req: &ModelRequest,
        onto: Increments<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.recorded(req, |model, first| {
            model.call_streaming(req, &mut |increment| {
                first(increment);
                onto(increment);
            })
        })
    }

    fn call_speculating(
        &mut self,
        req: &ModelRequest,
        onto: Increments<'_>,
        early: EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        self.recorded(req, |model, first| {
            model.call_speculating(
                req,
                &mut |increment| {
                    first(increment);
                    onto(increment);
                },
                early,
            )
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]
mod tests;
