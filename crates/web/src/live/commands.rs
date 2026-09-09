// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The commands a watcher can send: cancel, fork, steer.

use channels::{ClientFrame, RunId, Seq};

use crate::lang::{Msg, fill, say};

use super::feed::Line;

/// One line of the feed, said. The kind stays as the Ledger spells it -
/// it is an identifier, not a sentence - and only the description is
/// translated.
pub(crate) fn line_text(lang: crate::lang::Lang, line: &Line) -> String {
    match line.msg {
        Some(msg) => fill(say(lang, msg), &[("who", &line.who)]),
        None => format!("{:?} \u{b7} {}", line.kind, line.who),
    }
}

/// The run identifier, said.
pub(crate) fn run_id_line(lang: crate::lang::Lang, run: RunId) -> String {
    fill(say(lang, Msg::LiveRunId), &[("id", &run.to_string())])
}

/// What one turn spent in tokens, said. Absolute counts: the wire
/// carries no context window, so there is no denominator to divide by.
pub(crate) fn tokens_line(lang: crate::lang::Lang, used: channels::Used) -> String {
    fill(
        say(lang, Msg::TurnTokens),
        &[
            ("input", &used.input.get().to_string()),
            ("output", &used.output.get().to_string()),
        ],
    )
}

/// How much of a tool's output this row did not show, and where the rest
/// is. A window that hides without saying so is worse than no window.
pub(crate) fn cut_line(lang: crate::lang::Lang, cut: usize, at: Seq) -> String {
    fill(
        say(lang, Msg::TurnOutputCut),
        &[("cut", &cut.to_string()), ("seq", &at.value().to_string())],
    )
}

/// A checkpoint, shortened the way every git tool shortens one.
///
/// Seven characters, because that is what a person reads and quotes; the
/// whole oid is what the change list is addressed by and it never has to
/// be on screen for that.
pub(crate) fn short_oid(oid: channels::GitOid) -> String {
    let full = oid.to_string();
    full.chars().take(7).collect()
}

/// Where a branch would be taken from, said.
pub(crate) fn fork_line(lang: crate::lang::Lang, at: Seq) -> String {
    fill(
        say(lang, Msg::LiveForkFrom),
        &[("seq", &at.value().to_string())],
    )
}

/// What this page calls the session being watched, taken from the same
/// list the picker renders so the heading and the button cannot disagree.
pub(crate) fn named(runs: &[(RunId, String)], run: RunId) -> String {
    runs.iter()
        .find(|(id, _)| *id == run)
        .map_or_else(|| short_run(run), |(_, said)| said.clone())
}

/// A run's identifier, shortened for a button. The full one is in the
/// header of the page it opens, so nothing is lost by shortening here.
#[must_use]
pub fn short_run(run: RunId) -> String {
    let full = run.to_string();
    full.split('-').next().unwrap_or(&full).to_owned()
}

/// Stopping this session, from the page that shows it.
///
/// A run a person stopped ends as `cancelled` rather than `frozen`, and
/// the list draws the two differently because they call for different
/// next actions: one of them can be picked back up, and the other was
/// already finished with.
///
/// **`Takeover` is not here, and that is deliberate.** The verb is on
/// the wire and no city executes it, so a button for it would have had
/// exactly one outcome: a refusal. A control that cannot succeed is
/// worse than a missing one, because it spends a person's attention
/// teaching them the interface lies.
#[must_use]
pub fn cancel_command(run: RunId) -> ClientFrame {
    ClientFrame::Command(Box::new(channels::WireCommand::Cancel {
        idem: channels::IdemKey::derive(&run, Seq::FIRST, b"cancel"),
        run,
    }))
}

/// Branching a new run from a point in this one's history.
///
/// `at_seq` is the last record this page has seen for the run, which is
/// the only point a person watching it can actually mean. A fork records
/// a lineage and does not start driving by itself, so the button says
/// what it makes rather than what it starts.
#[must_use]
pub fn fork_command(run: RunId, at_seq: Seq) -> ClientFrame {
    ClientFrame::Command(Box::new(channels::WireCommand::Fork {
        idem: channels::IdemKey::derive(&run, at_seq, b"fork"),
        run,
        at_seq,
        addr: None,
    }))
}

/// A person's word into a running session. It arrives at the next safe
/// point rather than immediately, which is the difference between
/// steering a run and corrupting one.
#[must_use]
pub(crate) fn steer_command(run: RunId, text: &str) -> ClientFrame {
    ClientFrame::Command(Box::new(channels::WireCommand::Steer {
        idem: channels::IdemKey::derive(&run, Seq::FIRST, text.as_bytes()),
        run,
        text: text.to_owned(),
    }))
}
