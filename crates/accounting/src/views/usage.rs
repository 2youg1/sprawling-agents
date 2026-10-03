// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The usage table of every skill and every tool server, folded from the
//! whole ledger after the view lock is released
//! (`crates/accounting/spec/Views/Usage.lean` §8-34, accounting D49).
//!
//! What counts as a use is wire D33's: a skill read by a run whose
//! `run_started` pinned it, and a call whose recorded effect names a tool
//! server. The fold reads the ledger alone; the shelves and the
//! configuration are read once the fold is done, by [`UsageAsk::answer`].
//! The same pass hands every tool result to `runtime::ShellTally`, whose
//! reading is the shell table (wire D48).

use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{AuditVerdict, RunStarted, SkillAudited, SkillShelved};
use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{Address, B3Hash, Effect, EventKind, EventRecord, RunId, Seq, TimeMs};

use super::prepared::LedgerAsk;

mod answers;
mod export;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod model_tests;
mod shelves;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

pub(super) use shelves::Shelved;

/// The part of a skill a `describe` reads.
const GUIDE_PART: &str = "guide";
/// The part of a skill a `read` of its bare name reads.
const DOCUMENT_PART: &str = "SKILL.md";
/// The prefix the dormant index spells a skill with, which `describe`
/// also accepts.
const SKILL_PREFIX: &str = "skill ";

/// Which of the three usage questions a page asked.
pub enum UsageQuestion {
    Skills(Option<String>),
    Mcp(Option<String>),
    Export(wire::UsageKind, wire::ExportFormat),
    Shells,
}

/// What the usage questions need once the view lock is let go.
pub struct UsageAsk {
    pub(super) ledger: LedgerAsk,
    pub(super) question: UsageQuestion,
}

/// One call that used something, before its outcome is paired.
#[derive(Debug, Clone)]
struct Use {
    run: RunId,
    resident: Option<Address>,
    seq: Seq,
    at: TimeMs,
    call: String,
}

/// One read of a skill.
#[derive(Debug, Clone)]
struct SkillRead {
    name: String,
    part: String,
    digest: B3Hash,
    used: Use,
}

/// One call of a tool server's tool. `label` is the server the record's
/// effect names; a `call` line names none, and `tool` is then the whole
/// name it asked for.
#[derive(Debug, Clone)]
struct McpCall {
    label: Option<String>,
    tool: String,
    used: Use,
}

/// The fold: pure over the records it is given, so a replay of the same
/// ledger answers the same table.
#[derive(Debug, Default)]
pub(crate) struct Usage {
    pins: BTreeMap<RunId, BTreeMap<String, B3Hash>>,
    versions: BTreeMap<String, Vec<wire::SkillVersion>>,
    /// Every skill name a `skill_shelved` line has named so far.
    shelved: BTreeSet<String>,
    reads: Vec<SkillRead>,
    calls: Vec<McpCall>,
    audits: BTreeMap<String, Vec<(B3Hash, AuditVerdict, Seq)>>,
    outcomes: BTreeMap<(RunId, String), wire::UseOutcome>,
    shells: runtime::ShellTally,
}

impl UsageAsk {
    /// Folds the whole ledger, reads the shelves and the configuration,
    /// and answers. A ledger this city cannot index is "I could not look",
    /// with the reason (wire D47).
    pub(super) fn answer(self) -> wire::Answer {
        let usage = match self.ledger.usage() {
            Ok(usage) => usage,
            Err(stopped) => {
                return super::prepared::unavailable_because(
                    "Usage".to_owned(),
                    &stopped.into_ax(),
                );
            }
        };
        let city_root = &self.ledger.city_root;
        match self.question {
            UsageQuestion::Skills(only) => match shelves::shelved(city_root) {
                Some(shelved) => {
                    wire::Answer::SkillUsage(Box::new(usage.skills(&shelved, only.as_deref())))
                }
                None => super::prepared::unavailable("SkillUsage".to_owned()),
            },
            UsageQuestion::Shells => wire::Answer::Shells(Box::new(usage.shells())),
            UsageQuestion::Mcp(only) => wire::Answer::McpUsage(Box::new(
                usage.mcp(&shelves::configured(city_root), only.as_deref()),
            )),
            UsageQuestion::Export(what, format) => {
                let rows = match what {
                    wire::UsageKind::Skills => match shelves::shelved(city_root) {
                        Some(shelved) => export::skill_rows(&usage.skills(&shelved, None)),
                        None => return super::prepared::unavailable("UsageExport".to_owned()),
                    },
                    wire::UsageKind::Mcp => {
                        export::mcp_rows(&usage.mcp(&shelves::configured(city_root), None))
                    }
                };
                wire::Answer::UsageExport(Box::new(wire::UsageExportAnswer {
                    what,
                    format,
                    body: export::write(&rows, format),
                }))
            }
        }
    }
}

impl LedgerAsk {
    /// Every record from the first to the tail, folded. A line that will
    /// not read ends the fold rather than emptying it: what was read is
    /// still true, the rule `history` follows.
    fn usage(&self) -> Result<Usage, storage::StorageError> {
        let (index, dir) = self.indexed()?;
        let Some(tail) = index.tail_seq() else {
            return Ok(Usage::default());
        };
        let mut reader = index.reader(&dir);
        let mut next = Seq::FIRST.value();
        Ok(Usage::fold(std::iter::from_fn(|| {
            if next > tail.value() {
                return None;
            }
            let Ok(line) = reader.line_at(Seq::new(next)) else {
                return None;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                return None;
            };
            next = next.saturating_add(1);
            Some(record)
        })))
    }
}

impl Usage {
    /// Folds a run of records in ledger order.
    pub(crate) fn fold(records: impl IntoIterator<Item = impl Borrow<EventRecord>>) -> Usage {
        let mut usage = Usage::default();
        records
            .into_iter()
            .for_each(|record| usage.apply(record.borrow()));
        usage
    }

    /// Moves the table by one record. A payload that does not read as
    /// its kind adds nothing: the record is the writer's, and a guess at
    /// what it meant would be a use nobody made. Five kinds move it, so
    /// the test is by kind rather than a match over every kind there is.
    fn apply(&mut self, record: &EventRecord) {
        let kind = record.kind();
        if kind == EventKind::ToolResult
            && let Some(result) = record
                .data()
                .as_map()
                .get("result")
                .and_then(serde_json::Value::as_object)
        {
            self.shells.absorb(result);
        }
        if kind == EventKind::RunStarted
            && let Ok(started) = record.data().read::<RunStarted>()
        {
            self.pinned(record, started);
        } else if kind == EventKind::ToolCalled
            && let Ok(called) = record.data().read::<ToolCalled>()
        {
            self.called(record, &called);
        } else if kind == EventKind::ToolResult
            && let Ok(result) = record.data().read::<ToolResult>()
        {
            let outcome = match result.answer {
                ToolAnswer::Answered { .. } => wire::UseOutcome::Ok,
                ToolAnswer::Failed { .. } => wire::UseOutcome::Failed,
            };
            self.outcomes
                .insert((record.run(), result.tool_use_id), outcome);
        } else if kind == EventKind::SkillAudited
            && let Ok(audited) = record.data().read::<SkillAudited>()
        {
            self.audits.entry(audited.skill).or_default().push((
                audited.digest,
                audited.verdict,
                record.seq(),
            ));
        } else if kind == EventKind::SkillShelved
            && let Ok(shelved) = record.data().read::<SkillShelved>()
        {
            self.shelving(record, shelved);
        }
    }

    /// Files who put one content of a skill on its shelf: the version it
    /// names gains its author, or begins with this line (wire D33).
    fn shelving(&mut self, record: &EventRecord, shelved: SkillShelved) {
        let author = wire::VersionAuthor::Shelved {
            seq: record.seq(),
            from: shelved.source,
        };
        let versions = self.versions.entry(shelved.skill.clone()).or_default();
        match versions
            .iter_mut()
            .find(|version| version.digest == shelved.digest)
        {
            Some(version) => match version.author {
                wire::VersionAuthor::Shelved { .. } => {}
                wire::VersionAuthor::OutsideShelf | wire::VersionAuthor::Unrecorded => {
                    version.author = author;
                }
            },
            None => versions.push(wire::SkillVersion {
                digest: shelved.digest,
                seq: record.seq(),
                at: record.t(),
                run: record.run(),
                author,
            }),
        }
        self.shelved.insert(shelved.skill);
    }

    /// Files the skills one run was frozen with, and the first run that
    /// read each content no shelving line named: changed outside the
    /// city's doors when the name was shelved before, unrecorded when not.
    fn pinned(&mut self, record: &EventRecord, started: RunStarted) {
        let pins = self.pins.entry(record.run()).or_default();
        for pin in started.skills {
            let author = if self.shelved.contains(&pin.name) {
                wire::VersionAuthor::OutsideShelf
            } else {
                wire::VersionAuthor::Unrecorded
            };
            let versions = self.versions.entry(pin.name.clone()).or_default();
            if !versions.iter().any(|version| version.digest == pin.hash) {
                versions.push(wire::SkillVersion {
                    digest: pin.hash,
                    seq: record.seq(),
                    at: record.t(),
                    run: record.run(),
                    author,
                });
            }
            pins.insert(pin.name, pin.hash);
        }
    }

    /// Files one call as a skill read, a server call, or nothing.
    fn called(&mut self, record: &EventRecord, called: &ToolCalled) {
        let used = Use {
            run: record.run(),
            resident: record.addr().cloned(),
            seq: record.seq(),
            at: record.t(),
            call: called.id.clone(),
        };
        let name = called.name.as_str();
        let argument = |key: &str| {
            called
                .args
                .as_map()
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };
        if let Some(Effect::Connector { label }) = &called.effect {
            let tool = name
                .strip_prefix(label.as_str())
                .and_then(|rest| rest.strip_prefix('_'))
                .unwrap_or(name)
                .to_owned();
            self.calls.push(McpCall {
                label: Some(label.as_str().to_owned()),
                tool,
                used,
            });
        } else if name == runtime::tools::CallTool::NAME {
            if let Some(tool) = argument("name") {
                self.calls.push(McpCall {
                    label: None,
                    tool,
                    used,
                });
            }
        } else if let Some((skill, part)) = self.skill_part(record.run(), name, argument) {
            self.reads.push(SkillRead {
                digest: skill.1,
                name: skill.0,
                part,
                used,
            });
        }
    }

    /// The skill and the part of it one call read, when the run pinned
    /// that skill: wire D33 recognises the pin, never the spelling of a
    /// path alone.
    fn skill_part(
        &self,
        run: RunId,
        tool: &str,
        argument: impl Fn(&str) -> Option<String>,
    ) -> Option<((String, B3Hash), String)> {
        let pins = self.pins.get(&run)?;
        let pinned = |name: &str| pins.get(name).map(|hash| (name.to_owned(), *hash));
        if tool == runtime::tools::DescribeTool::NAME {
            let asked = argument("name")?;
            let asked = asked.trim();
            let skill = pinned(asked.strip_prefix(SKILL_PREFIX).unwrap_or(asked))?;
            return Some((skill, GUIDE_PART.to_owned()));
        }
        if tool == runtime::tools::ReadTool::NAME {
            let path = argument("path")?;
            if let Some(skill) = pinned(&path) {
                return Some((skill, DOCUMENT_PART.to_owned()));
            }
            let (head, rest) = path.split_once('/')?;
            return (!rest.is_empty())
                .then(|| pinned(head).map(|skill| (skill, rest.to_owned())))
                .flatten();
        }
        None
    }

    /// The outcome the call's paired result recorded.
    fn outcome(&self, used: &Use) -> wire::UseOutcome {
        self.outcomes
            .get(&(used.run, used.call.clone()))
            .copied()
            .unwrap_or(wire::UseOutcome::Unknown)
    }
}

/// Uses per UTC calendar day, oldest day first, the day read off
/// `runtime::clock::iso` so the calendar has one implementation.
fn per_day(moments: impl IntoIterator<Item = TimeMs>) -> Vec<wire::DayCount> {
    let mut days: BTreeMap<String, u32> = BTreeMap::new();
    for at in moments {
        let stamp = runtime::clock::iso(at);
        let day = stamp.get(..DAY_LEN).unwrap_or(&stamp).to_owned();
        let count = days.entry(day).or_default();
        *count = count.saturating_add(1);
    }
    days.into_iter()
        .map(|(day, count)| wire::DayCount { day, count })
        .collect()
}

/// `YYYY-MM-DD`, the head of an ISO 8601 stamp.
const DAY_LEN: usize = 10;
