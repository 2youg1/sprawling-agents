// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How one session's event arm writes in frames
//! (`crates/wire/spec/Server/Socket.lean` §8-47h): every record already
//! queued when it wakes becomes its own `Event` frame, and the socket is
//! flushed once. The model there proves that where the frames are cut
//! changes nothing the page is told, that records are told in seq order
//! and none twice, and that every seq not told as a record is inside a
//! told `Lagged` range.

use axum::extract::ws::{Message, WebSocket};
use futures_util::SinkExt as _;
use kernel::Seq;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;

use crate::frames::{Lagged, ServerFrame};
use crate::reception::{SessionState, Stream};
use crate::server::committed::Committed;

/// What one session took from its subscription: a record and its seq,
/// or a skip the subscription reported.
pub(crate) enum Arrival<R> {
    Record(Seq, R),
    Skipped,
}

/// What one session tells the page: a range to ask for, or a record.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Said<R> {
    Lagged(Lagged),
    Record(R),
}

/// The most arrivals one frame takes: a frame this long is written out
/// first, so a long burst does not starve the session's other arms.
pub(crate) const FRAME_RECORDS_MAX: usize = 256;

/// Whether the subscription is still open after a frame was drained.
pub(super) enum Ending {
    Open,
    Closed,
}

/// `first` and every arrival already queued behind it, at most
/// [`FRAME_RECORDS_MAX`], and whether the subscription closed meanwhile.
pub(super) fn drained(
    first: Arrival<Committed>,
    events: &mut broadcast::Receiver<Committed>,
) -> (Vec<Arrival<Committed>>, Ending) {
    let mut arrivals = vec![first];
    while arrivals.len() < FRAME_RECORDS_MAX {
        match events.try_recv() {
            Ok(committed) => arrivals.push(Arrival::Record(committed.record().seq(), committed)),
            Err(TryRecvError::Lagged(_)) => arrivals.push(Arrival::Skipped),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Closed) => return (arrivals, Ending::Closed),
        }
    }
    (arrivals, Ending::Open)
}

/// What a session in `phase` tells the page for one frame of arrivals,
/// and the stream state after it: each record in turn through
/// [`Stream::before`], each skip through [`Stream::skipped`]. Pure.
pub(crate) fn framed<R>(
    stream: Stream,
    phase: SessionState,
    arrivals: Vec<Arrival<R>>,
) -> (Vec<Said<R>>, Stream) {
    let live = phase == SessionState::Live;
    let mut said = Vec::with_capacity(arrivals.len());
    let after = arrivals
        .into_iter()
        .fold(stream, |stream, arrival| match arrival {
            Arrival::Record(seq, record) if live => {
                let (lag, next) = stream.before(seq);
                said.extend(lag.map(Said::Lagged));
                said.push(Said::Record(record));
                next
            }
            // A session that has not been welcomed is shown no stream.
            Arrival::Record(..) => stream,
            Arrival::Skipped => stream.skipped(live),
        });
    (said, after)
}

/// Feeds each thing said to the socket as its own frame, then flushes
/// once.
pub(super) async fn write_frame(
    socket: &mut WebSocket,
    said: Vec<Said<Committed>>,
) -> Result<(), ()> {
    for each in said {
        let text = match each {
            Said::Lagged(lag) => serde_json::to_string(&ServerFrame::Lagged(lag))
                .map_err(|_| ())?
                .into(),
            Said::Record(committed) => committed.frame(),
        };
        socket.feed(Message::Text(text)).await.map_err(|_| ())?;
    }
    socket.flush().await.map_err(|_| ())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use kernel::Seq;
    use proptest::prelude::*;

    use super::{Arrival, Said, framed};
    use crate::reception::{SessionState, Stream};

    /// The arrivals a subscription hands over for `ledger` when the
    /// records `skipped` marks are lost: a run of lost records is one
    /// `Skipped`, as a lagged `broadcast` reports it.
    fn arrivals(ledger: &[u64], skipped: &[bool]) -> Vec<Arrival<u64>> {
        let mut arrivals = Vec::new();
        let mut lost = false;
        for (seq, gone) in ledger.iter().zip(skipped) {
            match (gone, lost) {
                (true, true) => {}
                (true, false) => arrivals.push(Arrival::Skipped),
                (false, _) => arrivals.push(Arrival::Record(Seq::new(*seq), *seq)),
            }
            lost = *gone;
        }
        arrivals
    }

    /// `arrivals` cut into frames at `cuts` and told frame by frame.
    fn told_in_frames(
        arrivals: Vec<Arrival<u64>>,
        cuts: &[usize],
        phase: SessionState,
    ) -> (Vec<Said<u64>>, Stream) {
        let mut points: Vec<usize> = cuts.iter().map(|cut| (*cut).min(arrivals.len())).collect();
        points.sort_unstable();
        let mut said = Vec::new();
        let mut stream = Stream::opening();
        let mut rest = arrivals;
        let mut taken = 0;
        for point in points {
            let tail = rest.split_off(point - taken);
            let (told, next) = framed(stream, phase, rest);
            said.extend(told);
            stream = next;
            rest = tail;
            taken = point;
        }
        let (told, next) = framed(stream, phase, rest);
        said.extend(told);
        (said, next)
    }

    proptest! {
        /// `crates/wire/spec/Server/Socket.lean`: cutting the arrivals
        /// into frames anywhere tells what one frame tells
        /// (`framing_is_invisible`); a live session tells exactly the
        /// records it was handed, in seq order (`records_said`,
        /// `said_in_seq_order`); every seq between its first and last
        /// told record is a told record or inside a told `Lagged`
        /// (`every_seq_up_to_the_last_is_told`); a session not yet
        /// welcomed tells nothing (`nothing_said_before_welcome`).
        #[test]
        fn framing_tells_every_record_once_in_order(
            start in 0u64..3,
            gaps in proptest::collection::vec(1u64..4, 0..40),
            skipped in proptest::collection::vec(any::<bool>(), 40),
            cuts in proptest::collection::vec(0usize..48, 0..6),
        ) {
            let ledger: Vec<u64> = gaps
                .iter()
                .scan(start, |seq, gap| {
                    *seq += gap;
                    Some(*seq)
                })
                .collect();
            let handed = arrivals(&ledger, &skipped);
            let kept: Vec<u64> = ledger
                .iter()
                .zip(&skipped)
                .filter(|(_, gone)| !**gone)
                .map(|(seq, _)| *seq)
                .collect();

            let whole = framed(Stream::opening(), SessionState::Live, arrivals(&ledger, &skipped));
            let cut = told_in_frames(handed, &cuts, SessionState::Live);
            prop_assert_eq!(&cut, &whole);

            let records: Vec<u64> = whole
                .0
                .iter()
                .filter_map(|said| match said {
                    Said::Record(seq) => Some(*seq),
                    Said::Lagged(_) => None,
                })
                .collect();
            prop_assert_eq!(&records, &kept);
            prop_assert!(records.windows(2).all(|pair| pair[0] < pair[1]));

            if let (Some(first), Some(last)) = (records.first(), records.last()) {
                for k in *first..=*last {
                    let told = whole.0.iter().any(|said| match said {
                        Said::Record(seq) => *seq == k,
                        Said::Lagged(lag) => lag.from.value() <= k && k <= lag.to.value(),
                    });
                    prop_assert!(told, "seq {} between {} and {} was told nothing", k, first, last);
                }
            }

            let unwelcomed = framed(
                Stream::opening(),
                SessionState::AwaitingHello,
                arrivals(&ledger, &skipped),
            );
            prop_assert!(unwelcomed.0.is_empty());
        }
    }
}
