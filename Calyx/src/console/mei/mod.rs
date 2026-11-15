// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Mei backend: display-local identity, one JSONL converter, replay,
//! and the child process group (`crates/sprawling/spec/Console.lean` §8-174).

use crate::part::{Ending, Outcome};
use crate::scene::Entry;
use kernel::{RunId, TimeMs};
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::local_time;
use super::program_status::{Counts, Rest};

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

fn session_failure(
    action: &'static str,
    subject: impl Into<String>,
    path: impl Into<String>,
) -> String {
    session_failure_with_recovery(
        action,
        subject,
        path,
        "use /resume <jsonl> with a complete log, or /new to create a different file",
    )
}

fn session_failure_with_recovery(
    action: &'static str,
    subject: impl Into<String>,
    path: impl Into<String>,
    recovery: &'static str,
) -> String {
    let path = path.into();
    AdapterFailure {
        code: "E_MEI_SESSION",
        action,
        rule: "a Mei session is one explicitly named, complete JSONL message log",
        recovery,
        subject: subject.into(),
        path: path.clone(),
        value: path,
    }
    .render()
}

trait SessionFile: Read + Write + Seek {
    fn truncate(&mut self, length: u64) -> std::io::Result<()>;
}

impl SessionFile for std::fs::File {
    fn truncate(&mut self, length: u64) -> std::io::Result<()> {
        self.set_len(length)
    }
}

struct AppendFailure {
    error: std::io::Error,
    rollback: Option<std::io::Error>,
}

fn append_contents<F: SessionFile>(file: &mut F, lines: &[String]) -> Result<(), AppendFailure> {
    let original_len = file.seek(SeekFrom::End(0)).map_err(|error| AppendFailure {
        error,
        rollback: None,
    })?;
    let result = (|| -> std::io::Result<()> {
        if original_len > 0 {
            let mut last = [0_u8; 1];
            file.seek(SeekFrom::End(-1))?;
            file.read_exact(&mut last)?;
            if last[0] != b'\n' {
                file.write_all(b"\n")?;
            }
        }
        for line in lines {
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
        }
        file.flush()
    })();
    if let Err(error) = result {
        let rollback = file.truncate(original_len).err();
        return Err(AppendFailure { error, rollback });
    }
    Ok(())
}

/// The current explicitly visible JSONL. New files start empty; imported
/// files are validated by Mei before the next model or Python side effect.
pub(crate) struct MeiSession {
    path: PathBuf,
    history: Vec<String>,
    resume: bool,
}

impl MeiSession {
    pub(crate) fn create_in(directory: &Path, at: TimeMs) -> Result<MeiSession, String> {
        let counter = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(
            "mei-{}-{}-{counter}.jsonl",
            at.value(),
            std::process::id()
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        options.open(&path).map_err(|err| {
            session_failure(
                "create a new Mei session",
                err.to_string(),
                path.display().to_string(),
            )
        })?;
        Ok(MeiSession {
            path,
            history: Vec::new(),
            resume: false,
        })
    }

    pub(crate) fn import(path: &Path) -> Result<MeiSession, String> {
        let path = path.to_path_buf();
        let text = std::fs::read_to_string(&path).map_err(|err| {
            session_failure(
                "read the Mei resume log",
                err.to_string(),
                path.display().to_string(),
            )
        })?;
        Ok(MeiSession {
            path,
            history: text.lines().map(str::to_owned).collect(),
            resume: true,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn history(&self) -> &[String] {
        &self.history
    }

    pub(crate) fn resume_path(&self) -> Option<&Path> {
        self.resume.then_some(self.path.as_path())
    }

    /// Publishes only a successfully finished segment. A failure leaves
    /// memory unchanged and restores the original file length, or reports
    /// both the write and rollback failures (`Console.lean` §8-174 D111).
    pub(crate) fn append(&mut self, lines: &[String]) -> Result<(), String> {
        if lines.is_empty() {
            return Ok(());
        }
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(&self.path)
            .map_err(|err| {
                session_failure(
                    "open the current Mei session for append",
                    err.to_string(),
                    self.path.display().to_string(),
                )
            })?;
        append_contents(&mut file, lines).map_err(|failure| {
            if let Some(rollback) = failure.rollback {
                session_failure_with_recovery(
                    "roll back a failed Mei segment append",
                    format!(
                        "{}; restoring the original log also failed: {rollback}",
                        failure.error
                    ),
                    self.path.display().to_string(),
                    "keep the damaged file for manual recovery, use /new for further work, and resume only a complete backup",
                )
            } else {
                session_failure(
                    "append a successful Mei segment",
                    failure.error.to_string(),
                    self.path.display().to_string(),
                )
            }
        })?;
        self.history.extend_from_slice(lines);
        self.resume = true;
        Ok(())
    }
}

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
    usage: Option<String>,
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
            usage: None,
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

    pub(crate) fn offer(&self) -> String {
        let usage = self.usage.as_deref().unwrap_or("unknown");
        if self.model.is_empty() {
            usage.to_owned()
        } else {
            format!("{} · {usage}", self.model)
        }
    }

    pub(crate) fn line(&mut self, line: &str, at: TimeMs) -> Vec<Entry> {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![note(line)];
        };
        if value.get("type").and_then(Value::as_str) == Some("mei.obs") {
            return self.observation(&value);
        }
        if value.get("role").is_none() || value.get("content").is_none() {
            return vec![note(line)];
        }
        match value.get("role").and_then(Value::as_str) {
            Some("assistant") => self.assistant(value.get("content").unwrap_or(&Value::Null), at),
            Some("user") => self.tool_results(value.get("content").unwrap_or(&Value::Null), at),
            _ => vec![note(line)],
        }
    }

    fn observation(&mut self, value: &Value) -> Vec<Entry> {
        match value.get("kind").and_then(Value::as_str) {
            Some("usage") => {
                self.usage = value.get("usage").map(|usage| usage.to_string());
                Vec::new()
            }
            Some("pulse") => Vec::new(),
            _ => Vec::new(),
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

mod process;

#[cfg(test)]
#[allow(
    unused_imports,
    reason = "adapter_tests reaches mei_command through super"
)]
use process::mei_command;
pub(crate) use process::{AdapterFailure, MeiProcess};

pub fn verb(replay: Option<&str>, clock: super::terminal::Clock) -> std::process::ExitCode {
    super::ui::mei::run(replay, clock)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod session_tests;

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod tests;
