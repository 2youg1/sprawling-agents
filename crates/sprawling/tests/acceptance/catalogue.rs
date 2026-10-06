// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every built-in tool a run is offered is called once and answers the
//! way its own SPEC says (`crates/accounting/Spec.lean` section 2, the seventh and
//! eighth assertions).

use std::collections::BTreeSet;
use std::path::Path;

use accounting::worker::RunWorker;

use crate::city::{self, History};
use crate::episodes::{self, Episode, Observed, Setup};
use crate::script;

/// Rules that let the lead write where it works and give each room a
/// tree of its own, so `pr` has something to offer.
const REVIEWED: &str = "confidential = false\nwrite = \"everything\"\nreview = true\n";

/// One plan node ready to claim.
const PLAN: &str = "\
# Roadmap

| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
| 1 | wire the kiln | 1 |  | Not started |  |
";

#[test]
fn every_tool_a_builder_is_offered_is_called_and_answered() {
    let setup = &episodes::LAB;
    let (short, context) = run_the_script(setup, &episodes::for_builders(setup), |worker, dir| {
        city::raise(worker, setup.building, "minimal");
        city::rules(dir, setup.building, REVIEWED);
    });

    assert_eq!(short, Vec::<String>::new(), "{context}");
}

#[test]
fn every_tool_city_hall_is_offered_is_called_and_answered() {
    let setup = &episodes::HALL;
    // The founding raised City Hall; nothing more to raise.
    let (short, context) = run_the_script(setup, &episodes::for_city_hall(setup), |_, _| {});

    assert_eq!(short, Vec::<String>::new(), "{context}");
}

/// Everything the script's run left behind that a verdict reads.
struct Landing<'a> {
    city: &'a Path,
    setup: &'a Setup,
    offered: &'a [String],
    history: &'a History,
    rules_before: &'a [u8],
}

/// Founds a city, lets `raise` build what the setup needs, sends the
/// lead's room its task, and answers every shortfall of the coverage,
/// with what the model was offered and what the dispatch answered.
fn run_the_script(
    setup: &Setup,
    episodes: &[Episode],
    raise: impl FnOnce(&mut RunWorker, &Path),
) -> (Vec<String>, String) {
    let dir = tempfile::tempdir().unwrap();
    let (factory, offered, _) = script::scripted(steps_of(episodes));
    let (mut worker, ledger) = city::city_with_a_model(dir.path(), factory);
    raise(&mut worker, dir.path());
    let building = kernel::Address::parse(setup.building).unwrap();
    city::move_in(dir.path(), setup.room);
    city::move_in(dir.path(), setup.neighbour);
    std::fs::write(::city::roadmap_path(dir.path(), &building), PLAN).unwrap();
    episodes::lay_draft(dir.path(), setup);
    crate::web_search::supply(dir.path());
    let rules_before = std::fs::read(::city::rules_path(dir.path(), &building)).unwrap_or_default();
    let dispatched = city::dispatch(&mut worker, setup.room);

    let offered = offered.lock().unwrap().clone();
    let short = covered(
        &Landing {
            city: dir.path(),
            setup,
            offered: &offered,
            history: &History::read(&ledger),
            rules_before: &rules_before,
        },
        episodes,
    );
    (
        short,
        format!("offered {offered:?}; the dispatch answered {dispatched:?}"),
    )
}

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
fn covered(landing: &Landing<'_>, episodes: &[Episode]) -> Vec<String> {
    let history = landing.history;
    let Some(lead) = history.started().first().map(|started| started.run) else {
        return vec!["no run started".to_owned()];
    };
    let calls = history.calls_of(lead);
    let called: BTreeSet<&str> = calls.iter().map(|call| call.called.name.as_str()).collect();
    let never: Vec<&str> = landing
        .offered
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
            setup: landing.setup,
            city: landing.city,
            history,
            lead,
            rules_before: landing.rules_before,
            answer: &result.answer,
        };
        if let Err(why) = (episode.holds)(&seen) {
            short.push(format!("{}: {why}", episode.step.tool));
        }
    }
    short
}
