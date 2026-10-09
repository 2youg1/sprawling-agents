// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Real privacy writes on a disposable Windows runner
//! (`crates/sprawling/spec/Privacy.lean` §16).
//!
//! Every test here writes the machine it runs on, so every one is
//! ignored and starts by refusing any machine that is not a GitHub
//! Actions Windows runner started with `SPRAWLING_DISPOSABLE_PRIVACY=1`
//! (`.github/workflows/privacy-windows-acceptance.yml`). A person's
//! computer is never written by these tests.
//!
//! Each test drives the built binary's `privacy` verbs with a home of its
//! own, because the elevated child that writes machine scope is that
//! executable. The registry paths and task names below are this test's
//! own witness, read with `reg.exe` and `Get-ScheduledTask`, so a wrong
//! path in the controls table cannot vouch for itself.

#![cfg(windows)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// One control of each operation kind, with where its value lives as this
/// test reads it independently.
struct Witness {
    control: &'static str,
    place: Place,
    /// A value of the right type that the controls table never writes.
    other: &'static str,
}

enum Place {
    Registry {
        key: &'static str,
        name: &'static str,
        kind: &'static str,
    },
    Task {
        path: &'static str,
        name: &'static str,
    },
}

const HKLM_DWORD: Witness = Witness {
    control: "feedback_notifications",
    place: Place::Registry {
        key: r"HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection",
        name: "DoNotShowFeedbackNotifications",
        kind: "REG_DWORD",
    },
    other: "7",
};

const HKCU_DWORD: Witness = Witness {
    control: "start_launch_tracking",
    place: Place::Registry {
        key: r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
        name: "Start_TrackProgs",
        kind: "REG_DWORD",
    },
    other: "7",
};

const USER_ENVIRONMENT: Witness = Witness {
    control: "powershell_telemetry_optout",
    place: Place::Registry {
        key: r"HKCU\Environment",
        name: "POWERSHELL_TELEMETRY_OPTOUT",
        kind: "REG_SZ",
    },
    other: "0",
};

const TASK: Witness = Witness {
    control: "device_census_task",
    place: Place::Task {
        path: r"\Microsoft\Windows\Device Information\",
        name: "Device",
    },
    other: "",
};

/// Refuses every machine but a disposable runner, before anything runs.
fn disposable() {
    let set = |name: &str, value: &str| std::env::var(name).is_ok_and(|found| found == value);
    assert!(
        set("GITHUB_ACTIONS", "true")
            && set("RUNNER_OS", "Windows")
            && set("SPRAWLING_DISPOSABLE_PRIVACY", "1"),
        "these tests write the machine they run on; they run only on a disposable GitHub Actions Windows runner"
    );
}

/// A home of its own, and the binary's `privacy` verbs run under it.
struct Cli {
    home: tempfile::TempDir,
}

/// What one verb answered: its stdout lines, or the stable code its
/// refusal leads with.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    Done(String),
    Refused(String),
}

impl Cli {
    fn new() -> Self {
        disposable();
        Self {
            home: tempfile::tempdir().unwrap(),
        }
    }

    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
    fn run(&self, args: &[&str]) -> Answer {
        // boundary-ok: the elevated child that writes machine scope is this executable, so the production write path runs only through the built binary
        let output = Command::new(env!("CARGO_BIN_EXE_sprawling"))
            .args(args)
            .env("USERPROFILE", self.home.path())
            .output()
            .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        if output.status.success() {
            return Answer::Done(stdout.trim().to_owned());
        }
        let stderr = String::from_utf8(output.stderr).unwrap();
        let refusal: Value = serde_json::from_str(stderr.trim())
            .unwrap_or_else(|_| panic!("{args:?} refused without json: {stderr}"));
        let subject = find(&refusal, "subject")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{args:?}: no subject in {refusal}"));
        Answer::Refused(subject.split(':').next().unwrap_or_default().to_owned())
    }

    /// The control's line of `privacy inspect`.
    fn inspect(&self, control: &str) -> Value {
        match self.run(&["privacy", "inspect", control]) {
            Answer::Done(line) => serde_json::from_str(&line).unwrap(),
            Answer::Refused(code) => panic!("inspect {control} refused: {code}"),
        }
    }

    /// What `control` reads now, spelled as the verbs take it back.
    fn reading(&self, control: &str) -> String {
        let line = self.inspect(control);
        line.get("reading")
            .unwrap_or_else(|| panic!("{control} is unreadable: {line}"))
            .to_string()
    }

    fn apply(&self, control: &str) -> Answer {
        let expected = self.reading(control);
        self.run(&["privacy", "apply", control, &expected, "--json"])
    }

    fn restore(&self, control: &str) -> Answer {
        let expected = self.reading(control);
        self.run(&["privacy", "restore", control, &expected, "--json"])
    }

    fn status(&self) -> Value {
        match self.run(&["privacy", "status"]) {
            Answer::Done(line) => serde_json::from_str(&line).unwrap(),
            Answer::Refused(code) => panic!("status refused: {code}"),
        }
    }

    fn history(&self) -> PathBuf {
        self.home
            .path()
            .join(".sprawling")
            .join("privacy")
            .join("changes.jsonl")
    }
}

fn find<'v>(value: &'v Value, key: &str) -> Option<&'v Value> {
    match value {
        Value::Object(map) => map
            .get(key)
            .or_else(|| map.values().find_map(|inner| find(inner, key))),
        Value::Array(items) => items.iter().find_map(|inner| find(inner, key)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn done(answer: &Answer) -> Value {
    match answer {
        Answer::Done(line) => serde_json::from_str(line).unwrap(),
        Answer::Refused(code) => panic!("refused: {code}"),
    }
}

/// What the witness reads now, as `reg.exe` or `Get-ScheduledTask` says it.
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn witnessed(witness: &Witness) -> String {
    match witness.place {
        Place::Registry { key, name, .. } => {
            let output = Command::new("reg")
                .args(["query", key, "/v", name])
                .output()
                .unwrap();
            if !output.status.success() {
                return "absent".to_owned();
            }
            let text = String::from_utf8_lossy(&output.stdout);
            text.lines()
                .find_map(|line| {
                    let mut words = line.split_whitespace();
                    (words.next() == Some(name))
                        .then(|| words.skip(1).collect::<Vec<_>>().join(" "))
                })
                .unwrap_or_else(|| panic!("reg query printed no value line: {text}"))
        }
        Place::Task { path, name } => powershell(&format!(
            "(Get-ScheduledTask -TaskPath '{path}' -TaskName '{name}').State"
        )),
    }
}

/// Puts the witness to `value` (`None` deletes it) behind the product's back.
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn set_outside(witness: &Witness, value: Option<&str>) {
    match (&witness.place, value) {
        (Place::Registry { key, name, kind }, Some(data)) => run_tool(
            "reg",
            &["add", key, "/v", name, "/t", kind, "/d", data, "/f"],
        ),
        (Place::Registry { key, name, .. }, None) => {
            // Deleting a value that is not there is the state asked for.
            drop(
                Command::new("reg")
                    .args(["delete", key, "/v", name, "/f"])
                    .output(),
            );
        }
        (Place::Task { path, name }, Some("enable")) => {
            powershell(&format!(
                "Enable-ScheduledTask -TaskPath '{path}' -TaskName '{name}' | Out-Null"
            ));
        }
        (Place::Task { path, name }, Some(_) | None) => {
            powershell(&format!(
                "Disable-ScheduledTask -TaskPath '{path}' -TaskName '{name}' | Out-Null"
            ));
        }
    }
}

#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn run_tool(program: &str, args: &[&str]) {
    let status = Command::new(program).args(args).status().unwrap();
    assert!(status.success(), "{program} {args:?} failed");
}

#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
fn powershell(script: &str) -> String {
    let output = Command::new("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{script}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// The witness's state before a case, put back after it.
fn restore_outside(witness: &Witness, before: &str) {
    match witness.place {
        Place::Registry { .. } if before == "absent" => set_outside(witness, None),
        Place::Registry { .. } => {
            let data = before.strip_prefix("0x").map_or_else(
                || before.to_owned(),
                |hex| u32::from_str_radix(hex, 16).unwrap().to_string(),
            );
            set_outside(witness, Some(&data));
        }
        Place::Task { .. } if before == "Disabled" => set_outside(witness, None),
        Place::Task { .. } => set_outside(witness, Some("enable")),
    }
}

/// Where each test leaves its record for the workflow's artifact.
fn evidence(name: &str, lines: &[Value]) {
    let Ok(dir) = std::env::var("SPRAWLING_PRIVACY_EVIDENCE") else {
        return;
    };
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    std::fs::write(Path::new(&dir).join(format!("{name}.jsonl")), text).unwrap();
}

/// Apply then restore through the CLI, read back independently each time.
fn cycle(cli: &Cli, witness: &Witness, record: &mut Vec<Value>) {
    let original = witnessed(witness);
    let applied = done(&cli.apply(witness.control));
    let written = witnessed(witness);
    let restored = done(&cli.restore(witness.control));
    let back = witnessed(witness);
    record.push(serde_json::json!({
        "control": witness.control, "original": original, "applied": applied,
        "written": written, "restored": restored, "back": back,
    }));
    assert_eq!(applied["done"], "applied", "{}", witness.control);
    assert_ne!(written, original, "{}", witness.control);
    assert_eq!(restored["done"], "restored", "{}", witness.control);
    assert_eq!(back, original, "{}", witness.control);
}

/// Each operation kind applies and restores, from the value the runner
/// has and, for values, from a different value put there first.
#[test]
#[ignore = "writes the machine; runs only on a disposable Windows runner"]
fn each_operation_kind_applies_and_restores() {
    let mut record = Vec::new();
    for witness in [&HKLM_DWORD, &HKCU_DWORD, &USER_ENVIRONMENT, &TASK] {
        let cli = Cli::new();
        let before = witnessed(witness);
        cycle(&cli, witness, &mut record);
        if let Place::Registry { .. } = witness.place {
            set_outside(witness, None);
            cycle(&Cli::new(), witness, &mut record);
            set_outside(witness, Some(witness.other));
            cycle(&Cli::new(), witness, &mut record);
        }
        restore_outside(witness, &before);
        assert_eq!(witnessed(witness), before, "{}", witness.control);
    }
    evidence("operation_kinds", &record);
}

/// A value changed by someone else after the apply is not overwritten.
#[test]
#[ignore = "writes the machine; runs only on a disposable Windows runner"]
fn restore_refuses_a_value_changed_after_the_apply() {
    let mut record = Vec::new();
    for (witness, third) in [(&HKLM_DWORD, Some("5")), (&TASK, Some("enable"))] {
        let cli = Cli::new();
        let before = witnessed(witness);
        assert_eq!(done(&cli.apply(witness.control))["done"], "applied");
        set_outside(witness, third);
        let changed = witnessed(witness);
        let refused = cli.restore(witness.control);
        let after = witnessed(witness);
        record.push(serde_json::json!({
            "control": witness.control, "changed": changed,
            "restore": format!("{refused:?}"), "after": after,
        }));
        assert_eq!(refused, Answer::Refused("conflict".to_owned()));
        assert_eq!(after, changed);
        restore_outside(witness, &before);
    }
    evidence("conflict", &record);
}

/// A target that already reads the written value is neither written nor
/// recorded.
#[test]
#[ignore = "writes the machine; runs only on a disposable Windows runner"]
fn an_already_written_value_is_not_recorded() {
    let cli = Cli::new();
    let before = witnessed(&HKLM_DWORD);
    set_outside(&HKLM_DWORD, Some("1"));
    let answer = done(&cli.apply(HKLM_DWORD.control));
    let lines = std::fs::read_to_string(cli.history()).unwrap_or_default();
    let status = cli.status();
    restore_outside(&HKLM_DWORD, &before);
    evidence(
        "already_written",
        &[serde_json::json!({ "answer": answer, "history": lines, "status": status })],
    );
    assert_eq!(answer["done"], "already_written");
    assert!(lines.is_empty(), "{lines}");
    assert_eq!(status, serde_json::json!([]));
}

/// `restore-all` restores every change the history owns, one at a time.
#[test]
#[ignore = "writes the machine; runs only on a disposable Windows runner"]
fn restore_all_restores_every_owned_change() {
    let cli = Cli::new();
    let witnesses = [&HKLM_DWORD, &HKCU_DWORD];
    let before: Vec<String> = witnesses.iter().map(|witness| witnessed(witness)).collect();
    for witness in witnesses {
        assert_eq!(done(&cli.apply(witness.control))["done"], "applied");
    }
    let answer = cli.run(&["privacy", "restore-all", "--json"]);
    let after: Vec<String> = witnesses.iter().map(|witness| witnessed(witness)).collect();
    evidence(
        "restore_all",
        &[serde_json::json!({ "answer": format!("{answer:?}"), "before": before, "after": after })],
    );
    let Answer::Done(lines) = answer else {
        panic!("restore-all refused: {answer:?}");
    };
    assert_eq!(lines.lines().count(), 2, "{lines}");
    assert_eq!(after, before);
}

/// Every control, one at a time: apply, read back, restore, read back.
/// A restore that does not end where the control started fails the run,
/// and so does any unknown or rolled-back operation; an apply Windows
/// keeps from landing is recorded and does not.
#[test]
#[ignore = "writes the machine; runs only on a disposable Windows runner"]
fn every_control_applies_and_restores() {
    let cli = Cli::new();
    let Answer::Done(all) = cli.run(&["privacy", "inspect"]) else {
        panic!("inspect refused");
    };
    let controls: Vec<String> = all
        .lines()
        .map(|line| {
            let line: Value = serde_json::from_str(line).unwrap();
            line["control"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(controls.len(), 88);
    let mut record = Vec::new();
    let mut failures = Vec::new();
    for control in &controls {
        let before = cli.inspect(control);
        let applied = cli.apply(control);
        let (restored, after) = match &applied {
            Answer::Done(line) if line.contains("\"applied\"") => {
                let restored = cli.restore(control);
                (Some(restored), cli.inspect(control))
            }
            Answer::Done(_) | Answer::Refused(_) => (None, cli.inspect(control)),
        };
        let fine = match (&applied, &restored) {
            (Answer::Done(_), Some(Answer::Done(line))) => line.contains("\"restored\""),
            (Answer::Done(line), None) => line.contains("\"already_written\""),
            (Answer::Refused(code), None) => code == "not_applied" || code == "target_absent",
            (Answer::Done(_) | Answer::Refused(_), Some(_)) => false,
        } && before["reading"] == after["reading"];
        if !fine {
            failures.push(control.clone());
        }
        record.push(serde_json::json!({
            "control": control, "before": before["reading"], "apply": format!("{applied:?}"),
            "restore": format!("{restored:?}"), "after": after["reading"], "fine": fine,
        }));
    }
    evidence("sweep", &record);
    assert!(failures.is_empty(), "{failures:?}");
}
