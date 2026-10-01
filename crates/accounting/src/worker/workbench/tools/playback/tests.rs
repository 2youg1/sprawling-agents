// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use serde_json::json;

use super::*;
use crate::playback::tests::{addr, city};

/// The tool a resident of `lab` holds, working in `lab/a`.
fn tool(root: &Path) -> PlaybackTool {
    PlaybackTool::new(root, addr("lab"), addr("lab/a")).unwrap()
}

fn call(args: Value) -> ToolCall {
    ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse(ACTION).unwrap(),
        args: Payload::new(args.as_object().cloned().unwrap()).unwrap(),
    }
}

fn answered(root: &Path, args: Value) -> Result<Value, AxCode> {
    tool(root)
        .invoke(&call(args))
        .map(|outcome| Value::Object(outcome.result.as_map().clone()))
        .map_err(|err| *err.code())
}

fn exports(root: &Path) -> PathBuf {
    CityLayout::new(root).playback_exports().join("lab")
}

fn listed(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// A page that passes: a policy first, nothing from outside.
fn template(body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta http-equiv=\"Content-Security-Policy\" \
         content=\"default-src 'none'; style-src 'unsafe-inline'; connect-src 'none'; \
         base-uri 'none'; form-action 'none'\"><title>day</title>{BUNDLE_BLOCK}</head>\
         <body>{body}</body></html>"
    )
}

#[test]
fn a_resident_cannot_name_a_reader_or_widen_its_own() {
    let (dir, _) = city();
    assert_eq!(
        [
            answered(
                dir.path(),
                json!({"action": "export", "name": "day", "include_confidential": true})
            ),
            answered(
                dir.path(),
                json!({"action": "export", "name": "day", "reader": {"person": "included"}})
            ),
        ],
        [Err(AxCode::InvalidArgs), Err(AxCode::InvalidArgs)]
    );
    assert_eq!(listed(&exports(dir.path())), Vec::<String>::new());
}

#[test]
fn a_confidential_building_never_reaches_a_residents_export_and_the_export_checks() {
    let (dir, _) = city();
    let exported = answered(dir.path(), json!({"action": "export", "name": "day"})).unwrap();
    let bytes = std::fs::read(exports(dir.path()).join("day.json")).unwrap();
    let bundle: Value = serde_json::from_slice(&bytes).unwrap();
    let checked = answered(dir.path(), json!({"action": "check", "file": "day.json"})).unwrap();
    assert_eq!(
        (
            exported["file"].clone(),
            bundle["source"]["reader"].clone(),
            String::from_utf8(bytes).unwrap().contains("secret-plan"),
            bundle["withheld"]["buildings"].clone(),
            checked["structure"].clone(),
            checked["source"].clone(),
        ),
        (
            json!("day.json"),
            json!({"resident": "lab"}),
            false,
            json!([{"building": "vault", "reason": "confidential"}]),
            json!({"status": "passed"}),
            json!({"status": "passed"}),
        )
    );
}

#[test]
fn an_export_never_lands_over_another_and_a_failed_page_leaves_nothing() {
    let (dir, _) = city();
    let first = answered(dir.path(), json!({"action": "export", "name": "day"}));
    let kept = std::fs::read(exports(dir.path()).join("day.json")).unwrap();
    let again = answered(
        dir.path(),
        json!({"action": "export", "name": "day", "from": 3}),
    );
    let outward = answered(
        dir.path(),
        json!({
            "action": "export",
            "name": "outward",
            "page": template("<img src=\"https://example.com/a.png\">"),
        }),
    );
    assert_eq!(
        (
            first.map(|_| ()),
            again,
            outward,
            std::fs::read(exports(dir.path()).join("day.json")).unwrap() == kept,
            listed(&exports(dir.path())),
        ),
        (
            Ok(()),
            Err(AxCode::InvalidArgs),
            Err(AxCode::InvalidArgs),
            true,
            vec!["day.json".to_owned()],
        )
    );
}

#[test]
fn a_run_without_a_worktree_writes_a_page_into_the_city_and_names_only_its_own_exports() {
    let (dir, _) = city();
    let page = answered(
        dir.path(),
        json!({"action": "export", "name": "day-1", "page": template("<p>day</p>")}),
    )
    .unwrap();
    let elsewhere = answered(
        dir.path(),
        json!({"action": "check", "file": "../vault/day.json"}),
    );
    let checked = answered(dir.path(), json!({"action": "check", "file": "day-1.html"})).unwrap();
    assert_eq!(
        (
            page["file"].clone(),
            page["offline"].clone(),
            elsewhere,
            checked["offline"].clone(),
            checked["source"].clone(),
            listed(&exports(dir.path())),
        ),
        (
            json!("day-1.html"),
            json!({"status": "passed"}),
            Err(AxCode::InvalidArgs),
            json!({"status": "passed"}),
            json!({"status": "passed"}),
            vec!["day-1.html".to_owned()],
        )
    );
}

/// A resident's `day` is the same condition as the person's `--day`: the
/// export records it as the day's two ends.
#[test]
fn a_residents_day_is_recorded_as_its_two_ends() {
    let (dir, _) = city();
    let exported = answered(
        dir.path(),
        json!({"action": "export", "name": "epoch", "day": "1970-01-01"}),
    )
    .map(|_| {
        let bytes = std::fs::read(exports(dir.path()).join("epoch.json")).unwrap();
        let bundle: Value = serde_json::from_slice(&bytes).unwrap();
        (
            bundle["source"]["selection"]["since"].clone(),
            bundle["source"]["selection"]["until"].clone(),
        )
    });
    assert_eq!(exported, Ok((json!("0"), json!("86400000"))));
}
