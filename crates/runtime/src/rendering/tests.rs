// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{Rendering, ToolAnswer};
use kernel::{AxCode, AxError, Payload, ToolName};
use serde_json::json;

use super::{Copied, shown};

fn payload(value: serde_json::Value) -> Payload {
    Payload::of(&value).unwrap()
}

fn read() -> ToolName {
    ToolName::parse("read").unwrap()
}

fn answered(value: serde_json::Value) -> ToolAnswer {
    ToolAnswer::Answered {
        result: payload(value),
    }
}

/// Version 1 is what every line in a ledger written before renderers
/// existed was shown as, so it stays the payload's compact JSON for
/// every tool and for every refusal.
#[test]
fn version_one_draws_the_payload_json_the_model_read_before() {
    let result = json!({"path": "a.md", "bytes": 3, "text": "hi\n"});
    let refused = AxError::failure(AxCode::InvalidArgs, "read", "missing").with_recovery("pass it");
    for (name, answer, printed) in [
        (
            read(),
            answered(result.clone()),
            serde_json::to_string(&payload(result.clone())).unwrap(),
        ),
        (
            ToolName::parse("status").unwrap(),
            answered(result.clone()),
            serde_json::to_string(&payload(result)).unwrap(),
        ),
        (
            read(),
            ToolAnswer::Failed {
                error: Payload::of(&refused).unwrap(),
            },
            serde_json::to_string(&Payload::of(&refused).unwrap()).unwrap(),
        ),
    ] {
        assert_eq!(shown(Rendering::V1, &name, &answer).unwrap(), printed);
    }
}

/// A `version` an earlier ledger row carries is not drawn: edit is
/// admitted by read stamps and takes none
/// (`crates/runtime/spec/Tools/Edit.lean` §8-64).
#[test]
fn a_read_is_a_header_and_its_text_marked_every_ten_lines() {
    let text: String = (0..23).map(|n| format!("line {n}\n")).collect();
    let answer = answered(json!({
        "path": "lab/a.md", "version": "cas:b3-00", "total_lines": 40,
        "next_offset": 23, "bytes": text.len(), "text": text,
    }));
    let drawn = shown(Rendering::V2, &read(), &answer).unwrap();
    let mut expected = String::from("lab/a.md lines 0-22 of 40, next 23\n");
    for n in 0..23 {
        if n % 10 == 0 {
            expected.push_str(&format!("{n}|"));
        }
        expected.push_str(&format!("line {n}\n"));
    }
    assert_eq!(drawn, expected);
}

/// An interval that starts off a multiple of ten marks its first line,
/// and one that ran to the end names the total it implies.
#[test]
fn an_interval_marks_its_first_line_and_an_end_names_its_total() {
    let answer = answered(json!({
        "path": "a.md", "offset": 18, "bytes": 9, "text": "r\ns\nt\nu",
    }));
    assert_eq!(
        shown(Rendering::V2, &read(), &answer).unwrap(),
        "a.md lines 18-21 of 22\n18|r\ns\n20|t\nu"
    );
}

#[test]
fn an_empty_file_and_an_offset_past_the_end_say_which() {
    let empty = answered(json!({"path": "a.md", "total_lines": 0, "bytes": 0, "text": ""}));
    assert_eq!(
        shown(Rendering::V2, &read(), &empty).unwrap(),
        "a.md is empty\n"
    );
    let past = answered(json!({
        "path": "a.md", "offset": 9, "total_lines": 4, "bytes": 0, "text": "",
    }));
    assert_eq!(
        shown(Rendering::V2, &read(), &past).unwrap(),
        "a.md has 4 lines; offset 9 is past the end\n"
    );
}

/// A payload the read renderer has no reading for is still shown whole.
#[test]
fn a_payload_with_no_reading_is_drawn_as_its_json() {
    let odd = json!({"path": "a.md", "text": 7});
    assert_eq!(
        shown(Rendering::V2, &read(), &answered(odd.clone())).unwrap(),
        serde_json::to_string(&payload(odd)).unwrap()
    );
}

/// Only a mark that numbers the line it sits on, for one reading of
/// where the passage starts, comes off; a numbered line the file holds
/// stays, and so does a replacement line with a number nobody copied.
#[test]
fn only_marks_drawn_where_they_sit_come_off() {
    let copied = Copied::of("row 9\n10|row 10\n7|kept\n");
    assert_eq!(copied.starts().into_iter().collect::<Vec<_>>(), vec![9]);
    let (bare, taken) = copied.drawn_from(9).unwrap();
    assert_eq!(
        (bare.as_str(), taken.into_iter().collect::<Vec<_>>()),
        ("row 9\nrow 10\n7|kept\n", vec![10])
    );
    assert_eq!(Copied::of("fn a() {\n|x\n").starts().len(), 0);
    let numbers = [10].into_iter().collect();
    assert_eq!(
        Copied::of("10|row ten\n12|keep\n").without(&numbers),
        "row ten\n12|keep\n"
    );
}

/// What the read renderer draws, the edit side takes back to the file's
/// own bytes, for any interval of any text.
#[test]
fn every_drawn_body_unmarks_to_its_text() {
    let text: String = (0..57).map(|n| format!("  v{n} = {n};\r\n")).collect();
    for offset in [0u64, 3, 10, 49] {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let start = usize::try_from(offset).unwrap();
        let body: String = lines[start..].concat();
        let answer = answered(json!({"path": "a.rs", "offset": offset, "bytes": 0, "text": body}));
        let drawn = shown(Rendering::V2, &read(), &answer).unwrap();
        let (_, marked) = drawn.split_once('\n').unwrap();
        assert_eq!(Copied::of(marked).drawn_from(offset).unwrap().0, body);
    }
}

#[test]
fn a_refusal_is_one_line_led_by_its_code() {
    let refused = AxError::failure(AxCode::InvalidArgs, "read", "data/x.csv: no such file")
        .with_nearby(vec![
            "data/readings.csv".to_owned(),
            "data/y.csv".to_owned(),
        ])
        .with_recovery("check the path");
    let answer = ToolAnswer::Failed {
        error: Payload::of(&refused).unwrap(),
    };
    assert_eq!(
        shown(Rendering::V2, &read(), &answer).unwrap(),
        "E_INVALID_ARGS read: data/x.csv: no such file; check the path; nearby: \
         data/readings.csv, data/y.csv"
    );
    let again = AxError::failure(AxCode::InvalidArgs, "exec", "busy\nnow")
        .retriable()
        .with_recovery("wait");
    assert_eq!(
        shown(
            Rendering::V2,
            &ToolName::parse("exec").unwrap(),
            &ToolAnswer::Failed {
                error: Payload::of(&again).unwrap()
            }
        )
        .unwrap(),
        "E_INVALID_ARGS exec: busy now; wait; retry"
    );
}

/// A door's refusal reaches the model with all three of its parts: the
/// rule, what broke it, and what is open instead.
#[test]
fn a_door_refusal_carries_its_rule_its_violation_and_the_way_open() {
    let kernel::gate::GateOutcome::Deny { refusal } = kernel::gate::command(&kernel::TaintSet::of(
        kernel::TaintSource::new("arrival:github").unwrap(),
    )) else {
        panic!("a tainted run's command was allowed");
    };
    let drawn = shown(
        Rendering::V2,
        &ToolName::parse("exec").unwrap(),
        &ToolAnswer::Failed {
            error: Payload::of(&*refusal).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(
        drawn,
        format!(
            "E_TAINTED_ACTION run a command: exec; {recovery}; rule: {rule}; {violation}; instead: {alternative}",
            recovery = refusal.recovery(),
            rule = refusal.gate().unwrap().rule(),
            violation = refusal.gate().unwrap().violation(),
            alternative = refusal.gate().unwrap().alternative(),
        )
    );
}
