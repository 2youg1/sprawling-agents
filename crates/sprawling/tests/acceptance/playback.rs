// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `playback` city tool (sprawling-SPEC.md 8-132): its catalogue
//! episode, and two residents of one building - a reporter who writes
//! the day as a playback page, and an editor who checks it against the
//! city - with the report kept where the city keeps exports.

use kernel::event::record::ToolAnswer;
use kernel::layout::CityLayout;
use kernel::{Address, AxError, IdemKey, RunId, Seq};
use serde_json::{Map, Value, json};

use crate::city::{self, History};
use crate::episodes::{Episode, Observed};
use crate::script::{self, Step};

/// A page that passes the static check: the policy first in its head,
/// nothing it loads from outside, and the empty data block the export
/// fills.
fn template() -> String {
    format!(
        "<!doctype html><html><head><meta http-equiv=\"Content-Security-Policy\" \
         content=\"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; \
         connect-src 'none'; base-uri 'none'; form-action 'none'\"><meta charset=\"utf-8\">\
         <title>The day</title>{}</head><body><main id=\"top\"><h1>The day</h1>\
         <a href=\"#top\">top</a></main></body></html>",
        accounting::playback::BUNDLE_BLOCK
    )
}

/// The catalogue episode: an export of the bundle into this building's
/// playback exports.
pub(crate) fn episode() -> Episode {
    Episode {
        step: Step {
            tool: "playback",
            args: json!({"action": "export", "name": "catalogue"}),
        },
        holds: |seen: &Observed<'_>| {
            let result = answered(seen.answer)?;
            let kept = CityLayout::new(seen.city)
                .playback_exports()
                .join(seen.setup.building)
                .join("catalogue.json");
            if result.get("file") == Some(&json!("catalogue.json")) && kept.is_file() {
                Ok(())
            } else {
                Err(format!(
                    "the export answered {result:?}; {} is not there",
                    kept.display()
                ))
            }
        },
    }
}

const NEWSROOM_RULES: &str = "confidential = false\nwrite = \"everything\"\n";

#[test]
fn a_reporter_writes_the_day_and_an_editor_checks_it_against_the_city() {
    let dir = tempfile::tempdir().unwrap();
    let (reporter, _, _) = script::scripted(vec![Step {
        tool: "playback",
        args: json!({
            "action": "export",
            "name": "day-1",
            "building": "newsroom",
            "page": template(),
        }),
    }]);
    let (mut worker, ledger) = city::city_with_a_model(dir.path(), reporter);
    city::raise(&mut worker, "newsroom", "minimal");
    city::rules(dir.path(), "newsroom", NEWSROOM_RULES);
    city::move_in(dir.path(), "newsroom/reporter");
    city::move_in(dir.path(), "newsroom/editor");
    let wrote = dispatch(&mut worker, "newsroom/reporter");
    drop(worker);
    let (editor, _, _) = script::scripted(vec![Step {
        tool: "playback",
        args: json!({"action": "check", "file": "day-1.html"}),
    }]);
    let mut worker = city::open_worker(dir.path(), editor);
    let checked = dispatch(&mut worker, "newsroom/editor");
    drop(worker);

    let history = History::read(&ledger);
    let kept = CityLayout::new(dir.path())
        .playback_exports()
        .join("newsroom")
        .join("day-1.html");
    let page = std::fs::read(&kept).unwrap_or_default();
    let by_person = accounting::playback::check(&page, &accounting::playback::Asked::default());
    let report = result_at(&history, "newsroom/reporter");
    let check = result_at(&history, "newsroom/editor");
    assert_eq!(
        (
            wrote.map_err(|err| *err.code()),
            checked.map_err(|err| *err.code()),
            report.get("file").cloned(),
            report.get("offline").cloned(),
            ["structure", "source", "offline"].map(|item| check.get(item).cloned()),
            by_person.structure,
        ),
        (
            Ok(()),
            Ok(()),
            Some(json!("day-1.html")),
            Some(json!({"status": "passed"})),
            [
                Some(json!({"status": "passed"})),
                Some(json!({"status": "passed"})),
                Some(json!({"status": "passed"})),
            ],
            accounting::playback::Verdict::Passed,
        ),
        "the reporter heard {report:?}; the editor heard {check:?}"
    );
}

/// Sends `room` its task, keyed by the room so two dispatches in one
/// city are two.
fn dispatch(worker: &mut accounting::worker::RunWorker, room: &str) -> Result<(), AxError> {
    worker.handle(wire::Command::Dispatch {
        addr: Address::parse(room).unwrap(),
        task: "Look back at the newsroom's day with the playback tool.".to_owned(),
        goal: "the playback tool has answered".to_owned(),
        mode: kernel::Mode::Chat,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, room.as_bytes()),
        session: None,
        effort: None,
        model: None,
    })
}

/// What the first `playback` call of the run started at `room` was
/// answered with.
fn result_at(history: &History, room: &str) -> Map<String, Value> {
    history
        .started()
        .iter()
        .filter(|started| started.addr.as_ref().map(Address::as_str) == Some(room))
        .flat_map(|started| history.calls_of(started.run))
        .find(|call| call.called.name.as_str() == "playback")
        .and_then(|call| call.results.into_iter().next())
        .map(|result| match result.answer {
            ToolAnswer::Answered { result } => result.as_map().clone(),
            ToolAnswer::Failed { error } => error.as_map().clone(),
        })
        .unwrap_or_default()
}

fn answered(answer: &ToolAnswer) -> Result<&Map<String, Value>, String> {
    match answer {
        ToolAnswer::Answered { result } => Ok(result.as_map()),
        ToolAnswer::Failed { error } => Err(format!("failed: {:?}", error.as_map())),
    }
}
