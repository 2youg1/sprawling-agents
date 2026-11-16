// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Locator};

use crate::offload::{OffloadSite, tee};

pub mod diff;
pub mod filter;
pub mod key;
pub mod record;
pub(crate) mod scan;
pub(crate) mod stages;

pub use diff::SieveHistory;
pub use filter::{Filter, FilterTable};
pub use key::CommandKey;
pub use record::{
    PassReason, ResultOffloaded, SieveAccount, SieveRecord, Sieved, Stage, StageOutcome,
    StageReport,
};

use scan::{Priority, is_protected, priority};
use stages::Cuts;

pub const SIEVE_FLOOR: usize = 2048;

pub const MAX_TOTAL_LINES: usize = 240;
pub const HEAD_LINES: usize = 40;
pub const TAIL_LINES: usize = 40;
pub const MIDDLE_LINES: usize = 60;

pub struct SieveInput<'a> {
    pub key: &'a CommandKey,
    pub exit_code: Option<i64>,
    pub text: &'a str,
}

fn bytes_of(lines: &[String]) -> u64 {
    lines
        .iter()
        .map(|l| u64::try_from(l.len()).unwrap_or(u64::MAX).saturating_add(1))
        .fold(0u64, u64::saturating_add)
}

struct Draft {
    lines: Vec<String>,
    account: Vec<StageReport>,
}

impl Draft {
    fn step(&mut self, stage: Stage, pass: impl FnOnce(&[String]) -> Vec<String>) {
        let candidate = pass(&self.lines);
        self.accept(stage, candidate);
    }

    fn accept(&mut self, stage: Stage, candidate: Vec<String>) {
        let outcome = if candidate == self.lines {
            StageOutcome::Noop
        } else {
            let before = bytes_of(&self.lines);
            let after = bytes_of(&candidate);
            if after > before {
                StageOutcome::Rejected { grew_to: after }
            } else {
                self.lines = candidate;
                StageOutcome::Applied {
                    bytes_before: before,
                    bytes_after: after,
                }
            }
        };
        self.account.push(StageReport { stage, outcome });
    }

    fn unavailable(&mut self, stage: Stage, reason: &str) {
        self.account.push(StageReport {
            stage,
            outcome: StageOutcome::Unavailable {
                reason: reason.to_owned(),
            },
        });
    }
}

fn filter_marks(lines: &[String], filter: &Filter) -> Vec<Option<Priority>> {
    let mut marks: Vec<Option<Priority>> = vec![None; lines.len()];
    let mut carried: Option<Priority> = None;
    for (index, line) in lines.iter().enumerate() {
        let mark = if filter.keeps(line) {
            let head = priority(line)
                .unwrap_or(Priority::Warning)
                .min(Priority::Warning);
            carried = filter.group_until_blank.then_some(head);
            Some(head)
        } else if line.trim().is_empty() {
            carried = None;
            None
        } else {
            carried
        };
        if let Some(slot) = marks.get_mut(index) {
            *slot = mark;
        }
    }
    marks
}

fn normalised(lines: &[String], filter: &Filter) -> Vec<String> {
    let stripped = if filter.strip_ansi {
        stages::strip_ansi(lines)
    } else {
        lines.to_vec()
    };
    let folded = stages::fold_blank(&stripped);
    let exempt: Vec<bool> = filter_marks(&folded, filter)
        .iter()
        .map(Option::is_some)
        .collect();
    stages::dedup_templates(&folded, &exempt)
}

fn previous_lines(
    previous: Option<Locator>,
    site: &mut OffloadSite<'_>,
    filter: &Filter,
) -> Result<Vec<String>, &'static str> {
    let Some(Locator::Cas { hash, .. }) = previous else {
        return Err("no previous run of this command in this run");
    };
    let bytes = site
        .cas
        .get(&hash)
        .map_err(|_| "previous original is no longer in the CAS")?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    Ok(normalised(&lines, filter))
}

fn marks(lines: &[String], filter: &Filter) -> Vec<Option<Priority>> {
    lines
        .iter()
        .zip(filter_marks(lines, filter))
        .map(|(line, kept)| match (priority(line), kept) {
            (Some(own), Some(k)) => Some(own.min(k)),
            (own, kept) => own.or(kept),
        })
        .collect()
}

fn size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let tenths = bytes.saturating_mul(10).checked_div(1024).unwrap_or(0);
    format!(
        "{}.{} KiB",
        tenths.checked_div(10).unwrap_or(0),
        tenths.checked_rem(10).unwrap_or(0)
    )
}

fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len().saturating_sub(i)).checked_rem(3) == Some(0) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn sieve(
    input: SieveInput<'_>,
    table: &FilterTable,
    site: &mut OffloadSite<'_>,
    history: &mut SieveHistory,
) -> Result<Sieved, AxError> {
    let text = input.text;
    if text.len() <= SIEVE_FLOOR {
        return Ok(Sieved::Passed {
            text: text.to_owned(),
            reason: PassReason::BelowFloor,

            account: None,
        });
    }
    let pinned = tee(text.as_bytes(), site)?;
    let previous = history.previous(input.key).cloned();
    history.record(input.key.clone(), pinned.original.clone());
    let filter = table.lookup(input.key);
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let lines_in = u64::try_from(lines.len()).unwrap_or(u64::MAX);
    let mut draft = Draft {
        lines,
        account: Vec::with_capacity(7),
    };

    if filter.strip_ansi {
        draft.step(Stage::StripAnsi, stages::strip_ansi);
    } else {
        draft.unavailable(Stage::StripAnsi, "disabled by the filter");
    }
    draft.step(Stage::FoldBlank, stages::fold_blank);
    draft.step(Stage::DedupTemplate, |now| {
        let exempt: Vec<bool> = filter_marks(now, filter)
            .iter()
            .map(Option::is_some)
            .collect();
        stages::dedup_templates(now, &exempt)
    });
    match previous_lines(previous, site, filter) {
        Ok(before) => draft.step(Stage::DiffPrevious, |now| diff::only_changes(now, &before)),
        Err(reason) => draft.unavailable(Stage::DiffPrevious, reason),
    }
    draft.step(Stage::Filter, |now| {
        let kept: Vec<String> = now
            .iter()
            .filter(|l| filter.keeps(l) || is_protected(l) || !filter.drops(l))
            .cloned()
            .collect();
        if kept.iter().all(|l| l.trim().is_empty()) {
            vec![filter.when_empty(input.key, input.exit_code)]
        } else {
            kept
        }
    });
    draft.step(Stage::CutLongLine, stages::cut_long_lines);
    let cuts = Cuts {
        max_lines: MAX_TOTAL_LINES,
        head: filter
            .head
            .and_then(|h| usize::try_from(h).ok())
            .unwrap_or(HEAD_LINES),
        tail: filter
            .tail
            .and_then(|t| usize::try_from(t).ok())
            .unwrap_or(TAIL_LINES),
        middle: MIDDLE_LINES,
    };
    draft.step(Stage::Truncate, |now| {
        stages::truncate(now, &marks(now, filter), cuts)
    });

    let Draft { lines, account } = draft;
    let body = lines.join("\n");
    let bytes_in = u64::try_from(text.len()).unwrap_or(u64::MAX);
    let body_len = u64::try_from(body.len()).unwrap_or(u64::MAX);
    let lines_out = u64::try_from(lines.len()).unwrap_or(u64::MAX);
    let footer = format!(
        "[sieve: {} → {} lines, {} → {}, filter={}, rest at {}]",
        grouped(lines_in),
        grouped(lines_out),
        size(bytes_in),
        size(body_len),
        filter.id,
        pinned.rest_path
    );
    let out = format!("{body}\n{footer}");
    if out.len() > text.len() {
        return Ok(Sieved::Passed {
            text: text.to_owned(),
            reason: PassReason::NothingShrank,
            account: Some(SieveAccount {
                filter: filter.id.clone(),
                lines_in,

                lines_out: lines_in,
                stages: account,
            }),
        });
    }
    let bytes_out = u64::try_from(out.len()).unwrap_or(u64::MAX);
    Ok(Sieved::Cut(SieveRecord {
        text: out,
        original: pinned.original,
        rest_path: pinned.rest_path,
        filter: filter.id.clone(),
        lines_in,
        lines_out,
        bytes_in,
        bytes_out,
        stages: account,
    }))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    reason = "test code"
)]
mod tests;
