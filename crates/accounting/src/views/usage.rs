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
//! reading is the shell table (wire D48), and files every call by the
//! tool it names, which is the evidence `Query::Catalog` reads
//! ([`Usage::tool_evidence`], accounting D55).

use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};

use kernel::event::record::{AuditVerdict, CatalogDrift, RunStarted, SkillAudited, SkillShelved};
use kernel::event::record::{ToolAnswer, ToolCalled, ToolResult};
use kernel::{Address, B3Hash, Effect, EventKind, EventRecord, RunId, Seq, TimeMs};

use super::prepared::LedgerAsk;

mod answers;
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod audit_tests;
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

/// The part of a skill a `tool` `detail` reads.
const GUIDE_PART: &str = "guide";
/// The part of a skill a `read` of its bare name reads.
const DOCUMENT_PART: &str = "SKILL.md";
/// The prefix the dormant index spells a skill with, which `tool`
/// `detail` also accepts.
const SKILL_PREFIX: &str = "skill ";
/// The tool a ledger written before `tool` existed fetched a guide with;
/// its lines are history and still count as reads of the guide.
const EARLIER_DESCRIBE: &str = "describe";
/// The tool a ledger written before `tool` existed ran a dormant tool
/// through; an unresolved line of it still counts as a server call.
const EARLIER_CALL: &str = "call";
/// The argument a ledger written before `read` took Claude Code's names
/// named the file with; its lines keep it, because recorded calls are
/// never re-spelled (`crates/runtime/spec/Tools.lean` D115).
const EARLIER_READ_PATH: &str = "path";

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

/// Whether a call of `tool` is a `tool` call of `op`, or a call of the
/// tool an earlier build spelled that op as.
fn is_op(tool: &str, argument: &impl Fn(&str) -> Option<String>, op: &str, earlier: &str) -> bool {
    tool == earlier
        || (tool == runtime::tools::ToolTool::NAME && argument("op").as_deref() == Some(op))
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
/// effect names; an unresolved `tool` `run` line names none, and `tool`
/// is then the whole name it asked for.
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
    /// Every run that started at an address, in ledger order.
    started: Vec<(RunId, Address, TimeMs)>,
    /// The newest record time, which dates the 30-day evidence window.
    latest: Option<TimeMs>,
    /// Catalog drift in ledger order, with the building its envelope names.
    drifts: Vec<(Address, Seq, CatalogDrift)>,
    /// The tools each run called, each with the moment of its last call.
    /// A `tool` `run` line records the tool it stood for, so this counts
    /// capabilities rather than the door they were reached through.
    tool_calls: BTreeMap<RunId, BTreeMap<String, TimeMs>>,
}

/// How a building's runs used its tools and skills in the last 30 days.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CatalogEvidence {
    pub(crate) runs_seen: u32,
    pub(crate) by_tool: BTreeMap<String, (u32, TimeMs)>,
    pub(crate) by_skill: BTreeMap<String, (u32, TimeMs)>,
}

/// Thirty days in milliseconds; the window ends at the Ledger's latest record.
const EVIDENCE_WINDOW_MS: u64 = 2_592_000_000;

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
                Ok(shelved) => {
                    wire::Answer::SkillUsage(Box::new(usage.skills(&shelved, only.as_deref())))
                }
                Err(stopped) => {
                    super::prepared::unavailable_because("SkillUsage".to_owned(), &stopped)
                }
            },
            UsageQuestion::Shells => wire::Answer::Shells(Box::new(usage.shells())),
            UsageQuestion::Mcp(only) => wire::Answer::McpUsage(Box::new(
                usage.mcp(&shelves::configured(city_root), only.as_deref()),
            )),
            UsageQuestion::Export(what, format) => {
                let rows = match what {
                    wire::UsageKind::Skills => match shelves::shelved(city_root) {
                        Ok(shelved) => export::skill_rows(&usage.skills(&shelved, None)),
                        Err(stopped) => {
                            return super::prepared::unavailable_because(
                                "UsageExport".to_owned(),
                                &stopped,
                            );
                        }
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
    pub(super) fn usage(&self) -> Result<Usage, storage::StorageError> {
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
        self.latest = Some(self.latest.map_or(record.t(), |at| at.max(record.t())));
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
            if let Some(addr) = record.addr() {
                self.started.push((record.run(), addr.clone(), record.t()));
            }
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
        } else if kind == EventKind::DigestInvalidated
            && let Ok(drift) = record.data().read::<CatalogDrift>()
            && let Some(building) = record.addr()
        {
            self.drifts.push((building.clone(), record.seq(), drift));
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

    /// Run counts and last use inside the 30 days before the Ledger's
    /// newest record. Skills use their recorded reads; tools use calls.
    pub(crate) fn catalog_evidence(&self, building: &Address) -> CatalogEvidence {
        let within =
            |addr: &Address| city::Building::of(addr).is_ok_and(|held| held.addr() == building);
        let recent = |at: TimeMs| {
            self.latest.is_some_and(|latest| {
                latest.value().saturating_sub(at.value()) <= EVIDENCE_WINDOW_MS
            })
        };
        let runs: Vec<RunId> = self
            .started
            .iter()
            .filter(|(_, addr, at)| within(addr) && recent(*at))
            .map(|(run, _, _)| *run)
            .collect();
        let mut by_tool: BTreeMap<String, (u32, TimeMs)> = BTreeMap::new();
        for calls in runs.iter().filter_map(|run| self.tool_calls.get(run)) {
            for (tool, at) in calls.iter().filter(|(_, at)| recent(**at)) {
                let seen = by_tool.entry(tool.clone()).or_insert((0, *at));
                seen.0 = seen.0.saturating_add(1);
                seen.1 = seen.1.max(*at);
            }
        }
        let mut skill_runs: BTreeMap<String, BTreeSet<RunId>> = BTreeMap::new();
        let mut skill_last: BTreeMap<String, TimeMs> = BTreeMap::new();
        for read in self
            .reads
            .iter()
            .filter(|read| recent(read.used.at) && read.used.resident.as_ref().is_some_and(within))
        {
            skill_runs
                .entry(read.name.clone())
                .or_default()
                .insert(read.used.run);
            skill_last
                .entry(read.name.clone())
                .and_modify(|at| *at = (*at).max(read.used.at))
                .or_insert(read.used.at);
        }
        let by_skill = skill_last
            .into_iter()
            .map(|(name, at)| {
                let count = skill_runs
                    .get(&name)
                    .map_or(0, |runs| u32::try_from(runs.len()).unwrap_or(u32::MAX));
                (name, (count, at))
            })
            .collect();
        CatalogEvidence {
            runs_seen: u32::try_from(runs.len()).unwrap_or(u32::MAX),
            by_tool,
            by_skill,
        }
    }

    /// Every catalog difference recorded for one building, oldest first.
    pub(crate) fn catalog_drifts(&self, building: &Address) -> Vec<(Seq, CatalogDrift)> {
        self.drifts
            .iter()
            .filter(|(addr, _, _)| addr == building)
            .map(|(_, seq, drift)| (*seq, drift.clone()))
            .collect()
    }

    /// The verdict of the current skill content and whether it is stale.
    pub(crate) fn skill_audit(&self, name: &str, content: &B3Hash) -> (Option<AuditVerdict>, bool) {
        let audits = self.audits.get(name).map_or(&[][..], Vec::as_slice);
        match city::audit_state(content, audits) {
            city::AuditState::Audited { verdict, .. } => (Some(verdict), false),
            city::AuditState::Unaudited => (None, false),
            city::AuditState::Stale { audited } => {
                let verdict = audits
                    .iter()
                    .filter(|(digest, verdict, _)| {
                        digest == &audited && *verdict != AuditVerdict::Unreachable
                    })
                    .max_by_key(|(_, _, seq)| *seq)
                    .map(|(_, verdict, _)| *verdict);
                (verdict, true)
            }
        }
    }

    /// Files one call as a skill read, a server call, or nothing, and
    /// under the tool it names.
    fn called(&mut self, record: &EventRecord, called: &ToolCalled) {
        self.tool_calls
            .entry(record.run())
            .or_default()
            .insert(called.name.as_str().to_owned(), record.t());
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
        } else if is_op(name, &argument, runtime::tools::ToolTool::RUN, EARLIER_CALL) {
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
        if is_op(tool, &argument, "detail", EARLIER_DESCRIBE) {
            let asked = argument("name")?;
            let asked = asked.trim();
            let skill = pinned(asked.strip_prefix(SKILL_PREFIX).unwrap_or(asked))?;
            return Some((skill, GUIDE_PART.to_owned()));
        }
        if tool == runtime::tools::ReadTool::NAME {
            let path = argument(runtime::tools::ReadTool::FILE_PATH)
                .or_else(|| argument(EARLIER_READ_PATH))?;
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
