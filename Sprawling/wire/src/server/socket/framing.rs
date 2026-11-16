// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use axum::extract::ws::{Message, WebSocket};
use futures_util::SinkExt as _;
use kernel::Seq;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;

use crate::frames::{Lagged, ServerFrame};
use crate::reception::{SessionState, Stream};
use crate::server::committed::Committed;

pub(crate) enum Arrival<R> {
    Record(Seq, R),
    Skipped,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Said<R> {
    Lagged(Lagged),
    Record(R),
}

pub(crate) const FRAME_RECORDS_MAX: usize = 256;

pub(super) enum Ending {
    Open,
    Closed,
}

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

            Arrival::Record(..) => stream,
            Arrival::Skipped => stream.skipped(live),
        });
    (said, after)
}

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
