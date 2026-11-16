// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;

use accounting::worker::RunWorker;

use crate::city::{self, History};
use crate::episodes::{self, Episode, Observed, Setup};
use crate::script;

const REVIEWED: &str = "confidential = false\nwrite = \"everything\"\nreview = true\n";

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

    let (short, context) = run_the_script(setup, &episodes::for_city_hall(setup), |_, _| {});

    assert_eq!(short, Vec::<String>::new(), "{context}");
}

struct Landing<'a> {
    city: &'a Path,
    setup: &'a Setup,
    offered: &'a [String],
    history: &'a History,
    rules_before: &'a [u8],
}

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

fn covered(landing: &Landing<'_>, episodes: &[Episode]) -> Vec<String> {
    let history = landing.history;
    let Some(lead) = history.started().first().map(|started| started.run) else {
        return vec!["no run started".to_owned()];
    };
    let calls = history.calls_of(lead);
    let called: BTreeSet<&str> = calls.iter().map(|call| call.called.name.as_str()).collect();
    let mut short = Vec::new();
    let offered = unfolded(landing.offered, &calls, &mut short);
    let never: Vec<&str> = offered
        .iter()
        .map(String::as_str)
        .filter(|name| !called.contains(name))
        .collect();
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
            .find(|call| {
                call.called.name.as_str() == episode.step.tool
                    && serde_json::Value::Object(call.called.args.as_map().clone())
                        == episode.step.args
            })
            .or_else(|| {
                calls
                    .iter()
                    .find(|call| call.called.name.as_str() == episode.step.tool)
            })
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

fn unfolded(offered: &[String], calls: &[city::Call], short: &mut Vec<String>) -> Vec<String> {
    let resident: BTreeSet<&str> = offered
        .iter()
        .map(String::as_str)
        .filter(|name| !name.contains(' '))
        .collect();
    let mut names: Vec<String> = resident.iter().map(|name| (*name).to_owned()).collect();
    for line in offered.iter().filter(|name| name.contains(' ')) {
        let Some((namespace, count)) = line
            .strip_suffix(')')
            .and_then(|rest| rest.split_once(" ("))
        else {
            short.push(format!("an index line read {line:?}"));
            continue;
        };
        let asked = serde_json::json!({ "op": "list", "namespace": namespace });
        let Some(text) = calls
            .iter()
            .filter(|call| call.called.name.as_str() == "tool")
            .filter(|call| serde_json::Value::Object(call.called.args.as_map().clone()) == asked)
            .find_map(|call| call.results.first())
            .and_then(|result| crate::episodes::answered_text(&result.answer).ok())
        else {
            short.push(format!("namespace {namespace} was never listed"));
            continue;
        };
        let members: Vec<String> = text
            .lines()
            .filter_map(|member| member.strip_prefix("- "))
            .map(|member| member.split_once(':').map_or(member, |(name, _)| name))
            .filter(|name| !resident.contains(name))
            .map(str::to_owned)
            .collect();
        if members.len().to_string() != count {
            short.push(format!(
                "namespace {namespace} counted {count} and listed {members:?}"
            ));
        }
        names.extend(members);
    }
    names
}
