// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The table its rows land in: seats, listing, and readings.

use channels::Address;

use super::plan::ENDED_ROWS;
use crate::phase::Phase;

use crate::app::Snapshot;
use crate::lang::{Lang, Msg, fill, say};

/// One row of the table.
///
/// Flattened out of the snapshot rather than passed as a `RunRow`, so
/// the row holds exactly what it draws and a change to what a row shows
/// is visible here rather than spread through the markup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatRow {
    pub addr: Address,
    pub phase: Phase,
    /// The last thing the model said, or nothing when it has not spoken.
    pub said: Option<String>,
    pub turns: u32,
    /// What it has cost, where any call settled an amount.
    pub spent: Option<channels::UsdMicros>,
}

/// Every session this client knows of, in two lists: what is still
/// moving, and what has ended.
///
/// Sorted by phase first, so what waits on a person is at the top of the
/// first list. Inside a phase, newest first: a person scanning for the
/// thing they just sent finds it where they are already looking.
#[must_use]
pub fn listing(snapshot: &Snapshot) -> (Vec<SeatRow>, Vec<SeatRow>) {
    let mut in_flight = Vec::new();
    let mut ended = Vec::new();
    for (_, row) in snapshot.runs() {
        let Some(addr) = row.addr.clone() else {
            // A run with no address is a run this client saw start
            // somewhere it cannot name, and a row that cannot be opened
            // is a row that wastes the reader's click.
            continue;
        };
        let seat = SeatRow {
            addr,
            phase: row.phase,
            said: row.said.clone(),
            turns: row.turns,
            spent: row.spent,
        };
        if row.phase.in_flight() {
            in_flight.push((row.started_at_seq, seat));
        } else {
            ended.push((row.started_at_seq, seat));
        }
    }
    in_flight.sort_by_key(|(seq, row)| {
        (
            crate::phase::READING_ORDER
                .iter()
                .position(|held| *held == row.phase)
                .unwrap_or(usize::MAX),
            std::cmp::Reverse(*seq),
        )
    });
    ended.sort_by_key(|(seq, _)| std::cmp::Reverse(*seq));
    ended.truncate(ENDED_ROWS);
    (
        in_flight.into_iter().map(|(_, row)| row).collect(),
        ended.into_iter().map(|(_, row)| row).collect(),
    )
}

/// What a row says it has cost. Absent is not zero: a subscription
/// settles no amount, and printing `$0.00` would be this page inventing
/// the one fact nobody sent it.
#[must_use]
pub fn spent_of(lang: Lang, spent: Option<channels::UsdMicros>) -> String {
    match spent {
        Some(amount) => crate::readout::render_usd(amount),
        None => say(lang, Msg::SessionsUnpriced).to_owned(),
    }
}

/// The three counts the top bar carries, said.
#[must_use]
pub fn counts_said(lang: Lang, snapshot: &Snapshot) -> [String; 3] {
    let (running, waiting, buildings) = snapshot.counts();
    [
        (Msg::CountRunning, running),
        (Msg::CountWaiting, waiting),
        (Msg::CountBuildings, buildings),
    ]
    .map(|(msg, count)| fill(say(lang, msg), &[("n", &count.to_string())]))
}
