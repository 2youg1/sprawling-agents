// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every built-in tool a run is offered is called once and answers the
//! way its own SPEC says (accounting-SPEC.md section 2, the seventh and
//! eighth assertions).

use std::collections::BTreeSet;
use std::path::Path;

use crate::city::{self, History};
use crate::episodes::{self, Episode, Observed, Setup};
use crate::script;

/// Rules that let the lead write where it works and give each room a
/// tree of its own, so `pr` has something to offer.
const REVIEWED: &str = "confidential = false\nwrite = \"everything\"\nreview = true\n";

#[test]
fn every_tool_a_builder_is_offered_is_called_and_answered() {
    let dir = tempfile::tempdir().unwrap();
    let setup = &episodes::LAB;
    let episodes = episodes::for_builders(setup);
    let (factory, offered, _) = script::scripted(steps_of(&episodes));
    let (mut worker, ledger) = city::city_with_a_model(dir.path(), factory);
    city::raise(&mut worker, setup.building, "minimal");
    city::rules(dir.path(), setup.building, REVIEWED);
    stand_up(dir.path(), setup);
    let dispatched = city::dispatch(&mut worker, setup.room);

    let offered = offered.lock().unwrap().clone();
    assert_eq!(
        covered(
            dir.path(),
            setup,
            &offered,
            &History::read(&ledger),
            &episodes
        ),
        Vec::<String>::new(),
        "offered {offered:?}; the dispatch answered {dispatched:?}"
    );
}

#[test]
fn every_tool_city_hall_is_offered_is_called_and_answered() {
    let dir = tempfile::tempdir().unwrap();
    let setup = &episodes::HALL;
    let episodes = episodes::for_city_hall(setup);
    let (factory, offered, _) = script::scripted(steps_of(&episodes));
    let (mut worker, ledger) = city::city_with_a_model(dir.path(), factory);
    stand_up(dir.path(), setup);
    let dispatched = city::dispatch(&mut worker, setup.room);

    let offered = offered.lock().unwrap().clone();
    assert_eq!(
        covered(
            dir.path(),
            setup,
            &offered,
            &History::read(&ledger),
            &episodes
        ),
        Vec::<String>::new(),
        "offered {offered:?}; the dispatch answered {dispatched:?}"
    );
}

/// Gives the building a resident beside the lead's room, and a plan
/// with one node ready to claim.
fn stand_up(dir: &Path, setup: &Setup) {
    city::move_in(dir, setup.room);
    city::move_in(dir, setup.neighbour);
    std::fs::write(
        ::city::roadmap_path(dir, &kernel::Address::parse(setup.building).unwrap()),
        PLAN,
    )
    .unwrap();
}

const PLAN: &str = "\
# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | wire the kiln | 1 |  | Not started |  |
";

fn steps_of(episodes: &[Episode]) -> Vec<script::Step> {
    episodes
        .iter()
        .map(|episode| script::Step {
            tool: episode.step.tool,
            args: episode.step.args.clone(),
        })
        .collect()
}

/// Every way the lead's run fell short of the coverage, all at once:
/// a tool offered and never called, a call without exactly one result,
/// and each verdict that did not hold, prefixed with its tool.
fn covered(
    dir: &Path,
    setup: &Setup,
    offered: &[String],
    history: &History,
    episodes: &[Episode],
) -> Vec<String> {
    let Some(lead) = history.started().first().map(|started| started.run) else {
        return vec!["no run started".to_owned()];
    };
    let calls = history.calls_of(lead);
    let called: BTreeSet<&str> = calls.iter().map(|call| call.called.name.as_str()).collect();
    let never: Vec<&str> = offered
        .iter()
        .map(String::as_str)
        .filter(|name| !called.contains(name))
        .collect();
    let mut short = Vec::new();
    if !never.is_empty() {
        short.push(format!("offered and never called: {never:?}"));
    }
    for call in &calls {
        if call.results.len() != 1 {
            short.push(format!(
                "{} ({}): {} results",
                call.called.name.as_str(),
                call.called.id,
                call.results.len()
            ));
        }
    }
    for episode in episodes {
        let Some(result) = calls
            .iter()
            .find(|call| call.called.name.as_str() == episode.step.tool)
            .and_then(|call| call.results.first())
        else {
            short.push(format!("{}: never answered", episode.step.tool));
            continue;
        };
        let seen = Observed {
            setup,
            city: dir,
            history,
            lead,
            answer: &result.answer,
        };
        if let Err(why) = (episode.holds)(&seen) {
            short.push(format!("{}: {why}", episode.step.tool));
        }
    }
    short
}
