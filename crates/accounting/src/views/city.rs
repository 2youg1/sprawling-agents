// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The city page split at the view lock: the runs, the shut scopes and
//! the pursuits copied out of the fold, and the buildings listed and
//! their plans read once the views are released (sprawling-SPEC.md
//! 8-100).
//!
//! **Why the directory is listed here and not in the fold.** A building
//! is a directory a person or an agent can make without a record, so
//! the listing is read at every asking; under the view lock it made the
//! fold wait on the disk for every poll of the city page.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::{Address, PursuitState};

use super::holding::Views;
use super::lines::{buildings_of, summarize};
use crate::plan_view::{PlanView, plans_of};

/// What the city page takes under the view lock.
pub struct CityAsk {
    city_root: PathBuf,
    plans: Arc<Mutex<PlanView>>,
    /// The answer as the fold holds it, with no buildings and no
    /// pursuit lines yet.
    held: wire::CityAnswer,
    pursuits: Vec<(Address, String, PursuitState)>,
    in_flight: u32,
}

impl Views {
    /// Copies out what the city page needs from the fold: pure memory,
    /// so the fold waits only for the copy.
    pub(super) fn city_ask(&self) -> CityAsk {
        CityAsk {
            city_root: self.city_root.clone(),
            plans: Arc::clone(&self.plans),
            held: wire::CityAnswer {
                runs: self
                    .hot
                    .runs()
                    .map(|(run, hot)| summarize(*run, hot))
                    .collect(),
                active: self.hot.active_count(),
                frozen: self.hot.frozen_count(),
                buildings: Vec::new(),
                pursuits: Vec::new(),
                halted: self.governance.halted.iter().map(named).collect(),
                proved: None,
            },
            pursuits: self
                .pursuits
                .iter()
                .map(|(addr, (goal, state))| (addr.clone(), goal.clone(), *state))
                .collect(),
            in_flight: u32::try_from(self.hot.active_count()).unwrap_or(u32::MAX),
        }
    }
}

impl CityAsk {
    /// Lists the buildings, reads their plans and the plans the pursuits
    /// aim at, and answers.
    ///
    /// The verdict is computed here rather than on the page, so the stop
    /// condition has one authority: a client that worked out for itself
    /// whether a city had finished would be the second.
    pub(super) fn read(self) -> wire::Answer {
        let buildings = buildings_of(&self.city_root);
        let wanted: BTreeSet<Address> = buildings
            .iter()
            .chain(self.pursuits.iter().map(|(addr, _, _)| addr))
            .cloned()
            .collect();
        let mut plans = plans_of(&self.plans, &self.city_root, wanted);
        let pursuits = self
            .pursuits
            .into_iter()
            .map(|(addr, goal, state)| {
                let ready = plans
                    .get(&addr)
                    .map(|plan| plan.ready.as_slice())
                    .unwrap_or_default();
                wire::PursuitLine {
                    goal,
                    state,
                    verdict: kernel::pursuit::observe(state, ready, self.in_flight),
                    addr,
                }
            })
            .collect();
        let buildings = buildings
            .into_iter()
            .filter_map(|addr| {
                let plan = plans.remove(&addr)?;
                Some(wire::BuildingProgress {
                    addr,
                    progress: plan.progress,
                    problems: plan.problems,
                    blocked: plan.blocked,
                    ready: u32::try_from(plan.ready.len()).unwrap_or(u32::MAX),
                })
            })
            .collect();
        wire::Answer::City(wire::CityAnswer {
            buildings,
            pursuits,
            ..self.held
        })
    }
}

/// One shut scope in the shape a `halt` frame names it.
///
/// The ledger keeps `Scope` and its own spelling; a page is answered in
/// the vocabulary it would use to ask, so nothing on the other side has
/// to take a string apart to know which building it is looking at.
fn named(scope: &kernel::event::Scope) -> wire::HaltScope {
    match scope {
        kernel::event::Scope::City => wire::HaltScope::City,
        kernel::event::Scope::Building(addr) => wire::HaltScope::Building(addr.clone()),
        kernel::event::Scope::Workshop(addr) => wire::HaltScope::Workshop(addr.clone()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;

    fn proved(views: &Views) -> Option<kernel::Seq> {
        match views.city_ask().read() {
            wire::Answer::City(city) => city.proved,
            other => panic!("the city page answered {other:?}"),
        }
    }

    /// Views rebuilt from a history proved before they folded it say so;
    /// views watching a served city's proof say nothing until the halt
    /// holds a whole verdict, and then name their head.
    #[test]
    fn a_city_answer_says_the_history_is_proved_only_once_the_halt_says_so() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let ledger = kernel::layout::CityLayout::new(dir.path()).ledger();
        let mut views = Views::rebuild(&ledger).unwrap();
        let rebuilt = proved(&views);
        let halt = storage::ChainHalt::awaiting_proof();
        views.watch_proof(halt.clone());
        let awaiting = proved(&views);
        halt.prove();
        let whole = proved(&views);
        assert!(views.head().is_some());
        assert_eq!(
            (rebuilt, awaiting, whole),
            (views.head(), None, views.head())
        );
    }
}
