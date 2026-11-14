// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Mei backend: display-local identity, one JSONL converter, replay,
//! and the child process group (`crates/sprawling/spec/Console.lean` §8-174).

use std::io::{BufRead, Read, Write};
use std::process::{Child, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;

use console_ffi::part::{Ending, Outcome};
use console_ffi::scene::Entry;
use kernel::{RunId, TimeMs};
use serde_json::Value;

use super::local_time;
use super::program_status::{Counts, Rest};
use super::ui::Show;

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

/// What a Mei driver hands the terminal's one writer.
pub(crate) enum MeiEvent {
    Line(String),
    Finished {
        result: Result<String, String>,
        cancelled: bool,
    },
}

/// What the console knows about the one local Mei task.
pub(crate) struct Mei {
    run: Option<RunId>,
    model: String,
    since: Option<TimeMs>,
    calling: bool,
    rest: Rest,
    headed: bool,
    last_text: String,
}

impl Mei {
    pub(crate) fn new(model: String) -> Mei {
        Mei {
            run: None,
            model,
            since: None,
            calling: false,
            rest: Rest::Idle,
            headed: false,
            last_text: String::new(),
        }
    }

    pub(crate) fn begin(&mut self, at: TimeMs) {
        let counter = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
        let mut bytes = [0x4d, 0x65, 0x69, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        for (index, byte) in counter.to_be_bytes().into_iter().enumerate() {
            if let Some(slot) = bytes.get_mut(index.saturating_add(8)) {
                *slot = byte;
            }
        }
        self.run = Some(RunId::from_bytes(bytes));
        self.since = Some(at);
        self.calling = false;
        self.rest = Rest::Idle;
        self.headed = false;
        self.last_text.clear();
    }

    pub(crate) fn counts(&self) -> Counts {
        Counts {
            runs: u64::from(self.run.is_some()),
            approvals: 0,
        }
    }

    pub(crate) fn rest(&self) -> Rest {
        self.rest
    }

    pub(crate) fn since(&self) -> Option<TimeMs> {
        self.since
    }

    pub(crate) fn calling(&self) -> bool {
        self.calling
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }

    /// The one converter used by a real child's stderr and by replay.
    pub(crate) fn line(&mut self, line: &str, at: TimeMs) -> Vec<Entry> {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![note(line)];
        };
        if value.get("role").is_none() || value.get("content").is_none() {
            return vec![note(line)];
        }
        match value.get("role").and_then(Value::as_str) {
            Some("assistant") => self.assistant(value.get("content").unwrap_or(&Value::Null), at),
            Some("user") => self.tool_results(value.get("content").unwrap_or(&Value::Null), at),
            _ => vec![note(line)],
        }
    }

    fn assistant(&mut self, content: &Value, at: TimeMs) -> Vec<Entry> {
        let mut entries = Vec::new();
        if !self.headed {
            self.headed = true;
            entries.push(Entry::Head {
                at: local_time::local(at),
                resident: "Mei".to_owned(),
                facts: if self.model.is_empty() {
                    Vec::new()
                } else {
                    vec![self.model.clone()]
                },
            });
        }
        let Some(blocks) = content.as_array() else {
            let text = content.as_str().unwrap_or_default().to_owned();
            self.last_text = text.clone();
            if !text.is_empty() {
                entries.push(Entry::Reply { said: text });
            }
            return entries;
        };
        let mut reasoning = 0_usize;
        let mut text = String::new();
        for block in blocks {
            match block.get("type").and_then(Value::as_str) {
                Some("thinking") => {
                    reasoning = reasoning.saturating_add(
                        block
                            .get("thinking")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .chars()
                            .count(),
                    );
                }
                Some("text") => text.push_str(
                    block
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ),
                Some("tool_use") => self.calling = true,
                Some(_) | None => {}
            }
        }
        if reasoning > 0 {
            entries.push(Entry::Reasoning {
                characters: u64::try_from(reasoning).unwrap_or(u64::MAX),
            });
        }
        self.last_text = text.clone();
        if !text.is_empty() {
            entries.push(Entry::Reply { said: text });
        }
        entries
    }

    fn tool_results(&mut self, content: &Value, at: TimeMs) -> Vec<Entry> {
        let mut entries = Vec::new();
        for block in content.as_array().into_iter().flatten() {
            if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                continue;
            }
            entries.push(Entry::Tool {
                at: local_time::local(at),
                name: "python".to_owned(),
                subject: String::new(),
                took_us: None,
                outcome: Outcome::Answered,
            });
            let text = match block.get("content") {
                Some(Value::String(text)) => text.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            if !text.is_empty() {
                entries.push(note(&text));
            }
        }
        self.calling = false;
        entries
    }

    pub(crate) fn finished(&mut self, result: Result<String, String>, at: TimeMs) -> Vec<Entry> {
        let (ending, answer) = match result {
            Ok(answer) => {
                self.rest = Rest::Done;
                (Ending::Done, Some(answer))
            }
            Err(error) => {
                self.rest = Rest::Failed;
                (Ending::Limit, Some(error))
            }
        };
        let took_s = self.since.and_then(|since| {
            at.value()
                .checked_sub(since.value())
                .and_then(|millis| millis.checked_div(1_000))
        });
        self.run = None;
        self.since = None;
        self.calling = false;
        let mut entries = Vec::new();
        match answer.filter(|answer| !answer.is_empty()) {
            Some(answer) if ending == Ending::Done && answer != self.last_text => {
                entries.push(Entry::Reply { said: answer });
            }
            Some(error) if ending == Ending::Limit => entries.push(note(&error)),
            _ => {}
        }
        entries.push(Entry::Ended {
            at: local_time::local(at),
            ending,
            took_s,
        });
        entries
    }

    pub(crate) fn cancelled(&mut self, at: TimeMs) -> Vec<Entry> {
        self.rest = Rest::Idle;
        let took_s = self.since.and_then(|since| {
            at.value()
                .checked_sub(since.value())
                .and_then(|millis| millis.checked_div(1_000))
        });
        self.run = None;
        self.since = None;
        self.calling = false;
        vec![Entry::Ended {
            at: local_time::local(at),
            ending: Ending::Cancelled,
            took_s,
        }]
    }
}

fn note(said: &str) -> Entry {
    Entry::Note {
        said: said.to_owned(),
    }
}

/// One adapter failure: the stable code, the rule it broke, the exact
/// repair, and what failed — subject, full path and offending value.
pub(crate) struct AdapterFailure {
    pub(crate) code: &'static str,
    pub(crate) action: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) recovery: &'static str,
    pub(crate) subject: String,
    pub(crate) path: String,
    pub(crate) value: String,
}

/// Encodes text as a TOML string rather than Rust debug syntax, whose
/// control-character escapes are not valid TOML.
fn toml_string(text: &str) -> String {
    toml::Value::from(text).to_string()
}

impl AdapterFailure {
    /// The structured diagnostic the repository rule asks for: a `toml`
    /// fenced block carrying every field, ready for the terminal boundary.
    pub(crate) fn render(&self) -> String {
        format!(
            "```toml\ncode = {}\naction = {}\nsubject = {}\npath = {}\nvalue = {}\nrule = {}\nrecovery = {}\n```",
            toml_string(self.code),
            toml_string(self.action),
            toml_string(&self.subject),
            toml_string(&self.path),
            toml_string(&self.value),
            toml_string(self.rule),
            toml_string(self.recovery),
        )
    }
}

struct Signals {
    done: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

/// One running child and the driver which reaps it. The two signals travel
/// together so Esc and the driver agree on the terminal state.
pub(crate) struct MeiProcess {
    pid: u32,
    signals: Signals,
    driver: Option<std::thread::JoinHandle<()>>,
}

impl MeiProcess {
    pub(crate) fn spawn(prompt: &str, to: SyncSender<Show>) -> Result<MeiProcess, String> {
        let program = std::env::var_os("MEI_BIN").unwrap_or_else(|| "mei".into());
        MeiProcess::spawn_with_program(prompt, to, &program)
    }

    fn spawn_with_program(
        prompt: &str,
        to: SyncSender<Show>,
        program: &std::ffi::OsStr,
    ) -> Result<MeiProcess, String> {
        let mut command = child::command(program);
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| {
                let program = program.to_string_lossy().into_owned();
                AdapterFailure {
                    code: "E_MEI_CONFIG",
                    action: "start Mei",
                    rule: "MEI_BIN must name an executable Mei binary",
                    recovery: "set MEI_BIN to the built binary, for example `MEI_BIN=target/debug/mei`, or put `mei` on PATH",
                    subject: err.to_string(),
                    path: program.clone(),
                    value: program,
                }
                .render()
            })?;
        let pid = child.id();
        let signals = Signals {
            done: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let driver_signals = Signals {
            done: Arc::clone(&signals.done),
            cancelled: Arc::clone(&signals.cancelled),
        };
        let prompt = prompt.to_owned();
        let driver = std::thread::spawn(move || {
            drive(child, prompt, to, &driver_signals);
        });
        Ok(MeiProcess {
            pid,
            signals,
            driver: Some(driver),
        })
    }

    /// Requests group termination without waiting for the UI's event queue.
    pub(crate) fn request_stop(&mut self) -> Result<(), String> {
        if !self.signals.done.swap(true, Ordering::SeqCst) {
            self.signals.cancelled.store(true, Ordering::SeqCst);
            stop_group(self.pid)
        } else {
            Ok(())
        }
    }

    /// Stops the whole process group on Unix, then joins the driver.
    #[cfg(all(test, unix))]
    pub(crate) fn stop(&mut self) -> Result<(), String> {
        let stopped = self.request_stop();
        let reaped = self.reap();
        stopped.and(reaped)
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.driver
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
    }

    pub(crate) fn reap(&mut self) -> Result<(), String> {
        let Some(driver) = self.driver.take() else {
            return Ok(());
        };
        driver
            .join()
            .map_err(|_| "the Mei driver panicked".to_owned())
    }
}

impl Drop for MeiProcess {
    fn drop(&mut self) {
        if let Err(err) = self.request_stop() {
            eprintln!("stop Mei process group: {err}");
        }
        if let Err(err) = self.reap() {
            eprintln!("reap Mei process group: {err}");
        }
    }
}

fn drive(mut child: Child, prompt: String, to: SyncSender<Show>, signals: &Signals) {
    let result = drive_output(&mut child, &prompt, &to);
    let (status, answer) = match result {
        Ok((status, answer)) => (status, answer),
        Err(error) => {
            if let Err(stop) = stop_after_io_failure(&mut child) {
                eprintln!("stop Mei after an I/O failure: {stop}");
            }
            if let Err(wait) = child.wait() {
                eprintln!("reap Mei after an I/O failure: {wait}");
            }
            (None, Err(error))
        }
    };
    let result = match (status, answer) {
        (Some(status), Ok(answer)) if status.success() => Ok(answer),
        (Some(status), Ok(_)) => Err(format!("Mei exited with {status}")),
        (_, Err(error)) => Err(error),
        (None, Ok(answer)) => Ok(answer),
    };
    signals.done.store(true, Ordering::SeqCst);
    drop(to.send(Show::Mei(MeiEvent::Finished {
        result,
        cancelled: signals.cancelled.load(Ordering::SeqCst),
    })));
}

fn drive_output(
    child: &mut Child,
    prompt: &str,
    to: &SyncSender<Show>,
) -> Result<(Option<ExitStatus>, Result<String, String>), String> {
    let mut input = child
        .stdin
        .take()
        .ok_or_else(|| "open Mei stdin".to_owned())?;
    input
        .write_all(prompt.as_bytes())
        .map_err(|err| format!("write the prompt: {err}"))?;
    drop(input);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "open Mei stdout".to_owned())?;
    let reader = std::thread::spawn(move || -> Result<String, String> {
        let mut bytes = Vec::new();
        let mut stdout = stdout;
        stdout
            .read_to_end(&mut bytes)
            .map_err(|err| format!("read Mei stdout: {err}"))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    });
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "open Mei stderr".to_owned())?;
    for line in std::io::BufReader::new(stderr).lines() {
        let line = line.map_err(|err| format!("read Mei stderr: {err}"))?;
        if to.send(Show::Mei(MeiEvent::Line(line))).is_err() {
            return Err("the console stopped listening".to_owned());
        }
    }
    let status = child.wait().map_err(|err| format!("wait for Mei: {err}"))?;
    let answer = reader
        .join()
        .map_err(|_| "the stdout reader panicked".to_owned())?;
    Ok((Some(status), answer))
}

#[cfg(unix)]
fn stop_after_io_failure(child: &mut Child) -> Result<(), String> {
    stop_group(child.id())
}

#[cfg(not(unix))]
fn stop_after_io_failure(child: &mut Child) -> Result<(), String> {
    child.kill().map_err(|err| err.to_string())
}

#[cfg(unix)]
fn stop_group(pid: u32) -> Result<(), String> {
    let pid = i32::try_from(pid).map_err(|_| "invalid Mei process id".to_owned())?;
    let pid =
        rustix::process::Pid::from_raw(pid).ok_or_else(|| "invalid Mei process id".to_owned())?;
    rustix::process::kill_process_group(pid, rustix::process::Signal::KILL)
        .map_err(|err| format!("terminate Mei process group {pid}: {err}"))
}

#[cfg(not(unix))]
fn stop_group(_pid: u32) -> Result<(), String> {
    Err("Mei process groups require POSIX".to_owned())
}

/// The CLI entry owned by the existing console, not by Mei core.
pub fn verb(replay: Option<&str>) -> std::process::ExitCode {
    super::ui::mei::run(replay)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

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
            |_| Ok(serde_json::json!({"stop_reason": "end_turn", "content": "answer"})),
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
        assert_eq!(
            parsed.get("value").and_then(toml::Value::as_str),
            Some("bad\u{1b}\0\u{7f}")
        );
    }

    #[cfg(unix)]
    #[test]
    fn requesting_stop_lets_the_writer_drain_a_full_queue() {
        let (to, from) = std::sync::mpsc::sync_channel(1);
        to.send(Show::Resized).unwrap();
        let mut process = MeiProcess::spawn_with_program(
            "echo busy >&2\nsleep 30",
            to,
            std::ffi::OsStr::new("sh"),
        )
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
}
