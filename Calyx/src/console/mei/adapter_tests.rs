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

use super::super::ui::Show;
use super::mei_command;

use super::*;

#[test]
fn the_selected_jsonl_is_passed_to_the_process_interface() {
    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("chosen.jsonl");
    let command = mei_command(std::ffi::OsStr::new("mei"), Some(&session));
    let arguments: Vec<&std::ffi::OsStr> = command.get_args().collect();
    assert_eq!(arguments, ["--resume".as_ref(), session.as_os_str()]);
}

#[test]
fn messages_map_to_existing_entries_and_live_state() {
    let mut seen = Mei::new("provider/model".to_owned());
    seen.begin(TimeMs::new(1_000));
    assert!(seen.run.is_some());
    assert_eq!(seen.counts().runs, 1);
    let entries = seen.line(
        &serde_json::json!({"role": "assistant", "content": [
            {"type": "thinking", "thinking": "why", "signature": "s"},
            {"type": "text", "text": "working"},
            {"type": "tool_use", "name": "python", "id": "a", "input": {"code": "print(1)"}}
        ]})
        .to_string(),
        TimeMs::new(2_000),
    );
    assert_eq!(entries.len(), 3);
    assert!(matches!(entries[0], Entry::Head { .. }));
    assert!(matches!(entries[1], Entry::Reasoning { characters: 3 }));
    assert!(matches!(entries[2], Entry::Reply { .. }));
    assert!(seen.calling());
    let entries = seen.line(
        &serde_json::json!({"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "a", "content": "first\n[... truncated; full output: /tmp/out ...]\nlast"}
        ]}).to_string(),
        TimeMs::new(3_000),
    );
    assert_eq!(entries.len(), 2);
    assert!(matches!(
        entries[0],
        Entry::Tool {
            outcome: Outcome::Answered,
            ..
        }
    ));
    assert!(matches!(entries[1], Entry::Note { .. }));
    assert!(!seen.calling());
    let entries = seen.finished(Ok("done".to_owned()), TimeMs::new(4_000));
    assert!(matches!(entries[0], Entry::Reply { .. }));
    assert!(matches!(
        entries[1],
        Entry::Ended {
            ending: Ending::Done,
            took_s: Some(3),
            ..
        }
    ));
    assert_eq!(seen.counts().runs, 0);
}

#[test]
fn the_real_core_trace_feeds_the_console_projection() {
    let mut request = serde_json::json!({"messages": []});
    let mut trace = Vec::new();
    let answer = mei::agent::run(
        &mut request,
        |_, _| Ok(serde_json::json!({"stop_reason": "end_turn", "content": "answer"})),
        |_| Ok("unused".to_owned()),
        &mut trace,
    )
    .unwrap();
    assert_eq!(answer, "answer");
    let line = std::str::from_utf8(&trace).unwrap().trim_end();
    let mut seen = Mei::new("m".to_owned());
    assert!(matches!(
        seen.line(line, TimeMs::new(1)).as_slice(),
        [Entry::Head { .. }, Entry::Reply { .. }]
    ));
}

#[test]
fn replay_and_live_lines_use_identical_projection() {
    let line = serde_json::json!({"role": "assistant", "content": "answer"}).to_string();
    let mut live = Mei::new("m".to_owned());
    let mut replay = Mei::new("m".to_owned());
    assert_eq!(
        live.line(&line, TimeMs::new(1)),
        replay.line(&line, TimeMs::new(1))
    );
}

#[test]
fn plain_errors_and_failures_are_notes_not_replies() {
    let mut seen = Mei::new("m".to_owned());
    assert!(matches!(
        seen.line("first\nsecond", TimeMs::new(1)).as_slice(),
        [Entry::Note { .. }]
    ));
    seen.begin(TimeMs::new(1));
    let entries = seen.finished(Err("failed".to_owned()), TimeMs::new(2));
    assert!(matches!(entries[0], Entry::Note { .. }));
    assert!(matches!(
        entries[1],
        Entry::Ended {
            ending: Ending::Limit,
            ..
        }
    ));
}

#[test]
fn observations_update_offer_without_notes() {
    let mut seen = Mei::new("deepseek/flash".to_owned());
    assert_eq!(seen.offer(), "deepseek/flash · unknown");
    assert!(
        seen.line(r#"{"type":"mei.obs","kind":"pulse"}"#, TimeMs::new(1),)
            .is_empty()
    );
    assert!(
        seen.line(
            r#"{"type":"mei.obs","kind":"usage","usage":{"input_tokens":3}}"#,
            TimeMs::new(2),
        )
        .is_empty()
    );
    assert_eq!(seen.offer(), "deepseek/flash · {\"input_tokens\":3}");
}

#[cfg(target_os = "linux")]
#[test]
fn an_io_failure_stops_the_whole_process_group() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("child-pid");
    let (to, from) = std::sync::mpsc::sync_channel(2);
    drop(from);
    let prompt = format!(
        "sleep 30 &\necho $! > {}\necho closed >&2\nwait\n",
        pid_file.display()
    );
    let mut process =
        MeiProcess::spawn_with_program(&prompt, to, std::ffi::OsStr::new("sh")).unwrap();
    process.reap().unwrap();
    let pid = std::fs::read_to_string(pid_file).unwrap();
    let stat = format!("/proc/{}/stat", pid.trim());
    let gone = || match std::fs::read_to_string(&stat) {
        Ok(stat) => stat
            .rsplit(") ")
            .next()
            .is_some_and(|rest| rest.starts_with('Z')),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => true,
        Err(err) => panic!("read {stat}: {err}"),
    };
    for _ in 0..200 {
        if gone() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(gone(), "the ordinary grandchild survived its driver");
}

#[test]
fn a_missing_mei_binary_is_a_structured_diagnostic() {
    let (to, _from) = std::sync::mpsc::sync_channel(1);
    let error = match MeiProcess::spawn_with_program(
        "prompt",
        to,
        std::ffi::OsStr::new("mei-definitely-not-on-path-xyz"),
    ) {
        Ok(_) => panic!("a missing binary must not spawn"),
        Err(error) => error,
    };
    assert!(error.contains("```toml"), "{error}");
    assert!(error.contains("E_MEI_CONFIG"), "{error}");
    assert!(error.contains("mei-definitely-not-on-path-xyz"), "{error}");
    assert!(error.contains("MEI_BIN"), "{error}");
}

#[test]
fn adapter_diagnostics_with_control_characters_are_parseable_toml() {
    let rendered = AdapterFailure {
        code: "E_MEI_CONFIG",
        action: "start Mei",
        rule: "MEI_BIN must name an executable Mei binary",
        recovery: "set MEI_BIN to a working executable",
        subject: "invalid program".to_owned(),
        path: "MEI_BIN".to_owned(),
        value: "bad\u{1b}\0\u{7f}".to_owned(),
    }
    .render();
    let document = rendered
        .strip_prefix("```toml\n")
        .and_then(|body| body.strip_suffix("\n```"))
        .unwrap();
    let parsed: toml::Table = toml::from_str(document).unwrap();
    let cause = parsed.get("cause").and_then(toml::Value::as_str).unwrap();
    assert!(cause.contains("path = MEI_BIN"), "{cause}");
    assert!(cause.contains("value = bad"), "{cause}");
    assert!(rendered.contains(r"\u001B"), "{rendered}");
    assert!(rendered.contains(r"\u0000"), "{rendered}");
    assert!(rendered.contains(r"\u007F"), "{rendered}");
}

#[cfg(unix)]
#[test]
fn requesting_stop_lets_the_writer_drain_a_full_queue() {
    let (to, from) = std::sync::mpsc::sync_channel(1);
    to.send(Show::Resized).unwrap();
    let mut process =
        MeiProcess::spawn_with_program("echo busy >&2\nsleep 30", to, std::ffi::OsStr::new("sh"))
            .unwrap();
    process.request_stop().unwrap();
    let mut cancelled = false;
    for _ in 0..3 {
        if let Show::Mei(MeiEvent::Finished {
            cancelled: was_cancelled,
            ..
        }) = from.recv().unwrap()
        {
            cancelled = was_cancelled;
            break;
        }
    }
    process.reap().unwrap();
    assert!(cancelled);
}

#[cfg(unix)]
#[test]
fn stopping_a_process_group_reaps_its_driver() {
    let (to, from) = std::sync::mpsc::sync_channel(2);
    let mut process =
        MeiProcess::spawn_with_program("sleep 30", to, std::ffi::OsStr::new("sh")).unwrap();
    process.stop().unwrap();
    let Show::Mei(MeiEvent::Finished { cancelled, .. }) = from.recv().unwrap() else {
        panic!("missing process result");
    };
    assert!(cancelled);
}

#[cfg(windows)]
#[test]
fn stopping_a_windows_job_reaps_its_driver() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("sleep.cmd");
    std::fs::write(&script, "@echo busy>&2\r\nping -n 40 127.0.0.1 >nul\r\n").unwrap();
    let (to, from) = std::sync::mpsc::sync_channel(2);
    let mut process = MeiProcess::spawn_with_program("", to, script.as_os_str()).unwrap();
    process.stop().unwrap();
    let Show::Mei(MeiEvent::Finished { cancelled, .. }) = from.recv().unwrap() else {
        panic!("missing process result");
    };
    assert!(cancelled);
}
