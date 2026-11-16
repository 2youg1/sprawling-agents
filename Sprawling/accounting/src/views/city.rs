// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::{Address, PursuitState};

use super::holding::Views;
use super::lines::{buildings_of, summarize};
use crate::plan_view::{PlanView, plans_of};

pub struct CityAsk {
    city_root: PathBuf,
    plans: Arc<Mutex<PlanView>>,

    held: wire::CityAnswer,
    pursuits: Vec<(Address, String, PursuitState)>,
    in_flight: u32,
}

impl Views {

    pub fn city(&self) -> Option<Address> {
        self.city
            .clone()
            .or_else(|| kernel::layout::CityLayout::new(&self.city_root).city_address())
    }

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

                proved: match &self.proof {
                    Some(halt) if !halt.proved() => None,
                    Some(_) | None => self.head,
                },
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

    pub(super) fn read(self) -> wire::Answer {
        let Ok(buildings) = buildings_of(&self.city_root) else {
            return super::prepared::unavailable("CityView".to_owned());
        };
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
        let ruled = buildings
            .iter()
            .map(|addr| {
                city::Building::of(addr).and_then(|held| city::has_rules(&self.city_root, &held))
            })
            .collect::<Result<Vec<bool>, kernel::AxError>>();
        let ruled = match ruled {
            Ok(ruled) => ruled,
            Err(stopped) => {
                return super::prepared::unavailable_because("CityView".to_owned(), &stopped);
            }
        };
        let buildings = buildings
            .into_iter()
            .zip(ruled)
            .filter_map(|(addr, ruled)| {
                let plan = plans.remove(&addr)?;
                Some(wire::BuildingProgress {
                    addr,
                    progress: plan.progress,
                    problems: plan.problems,
                    blocked: plan.blocked,
                    ready: u32::try_from(plan.ready.len()).unwrap_or(u32::MAX),
                    adoptable: !ruled,
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

fn named(scope: &kernel::event::Scope) -> wire::HaltScope {
    match scope {
        kernel::event::Scope::City => wire::HaltScope::City,
        kernel::event::Scope::Building(addr) => wire::HaltScope::Building(addr.clone()),
        kernel::event::Scope::Workshop(addr) => wire::HaltScope::Workshop(addr.clone()),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn proved(views: &Views) -> Option<kernel::Seq> {
        match views.city_ask().read() {
            wire::Answer::City(city) => city.proved,
            other => panic!("the city page answered {other:?}"),
        }
    }

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
