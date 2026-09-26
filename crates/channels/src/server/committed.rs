// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A committed record and its one wire frame (channels-SPEC.md 8-47).
//!
//! The frame is spelled once, before the broadcast, so a socket's share
//! of an event is two reference counts and a write however many tabs a
//! person has open.

use std::sync::Arc;

use axum::extract::ws::Utf8Bytes;
use kernel::{AxCode, AxError, EventRecord};

/// What the Ledger's writer broadcasts: a record, and the
/// `ServerFrame::Event` text that carries it. Cloning copies no bytes.
#[derive(Debug, Clone)]
pub struct Committed {
    record: Arc<EventRecord>,
    frame: Utf8Bytes,
}

impl Committed {
    /// Spells the frame: `{"event":`, the record's JSON, `}` - the shape
    /// the externally tagged `ServerFrame::Event` serialises to.
    ///
    /// # Errors
    /// `WireMismatch` when the record does not serialise; that record
    /// reaches no socket, and the gap in `seq` it leaves makes each live
    /// session send `Lagged` at the next record (channels-SPEC.md 8-41).
    pub fn new(record: EventRecord) -> Result<Self, AxError> {
        let unframed = |detail: String| {
            AxError::failure(AxCode::WireMismatch, "frame a committed record", detail)
                .with_recovery("ask for the record's range; the ledger holds it")
        };
        let mut frame = b"{\"event\":".to_vec();
        serde_json::to_writer(&mut frame, &record)
            .map_err(|source| unframed(source.to_string()))?;
        frame.push(b'}');
        let text = String::from_utf8(frame).map_err(|source| unframed(source.to_string()))?;
        Ok(Self {
            record: Arc::new(record),
            frame: Utf8Bytes::from(text),
        })
    }

    /// The record the frame carries.
    #[must_use]
    pub fn record(&self) -> &EventRecord {
        &self.record
    }

    /// The frame's text, shared rather than copied.
    pub(crate) fn frame(&self) -> Utf8Bytes {
        self.frame.clone()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::arithmetic_side_effects
)]
mod tests {
    use std::time::Instant;

    use kernel::{B3Hash, EventRecord, Seq, TimeMs};
    use tokio::sync::broadcast;

    use super::Committed;
    use crate::wire::ServerFrame;

    fn record(seq: u64) -> EventRecord {
        EventRecord::from_draft(
            kernel::EventDraft {
                run: kernel::RunId::from_bytes([4u8; 16]),
                t: TimeMs::new(seq),
                who: "resident".to_owned(),
                addr: None,
                kind: kernel::EventKind::RunStarted,
                data: kernel::Payload::empty(),
                ig: false,
            },
            Seq::new(seq),
            B3Hash::digest(b"prev"),
        )
    }

    #[test]
    fn the_spliced_frame_is_the_serialised_event_frame() {
        let committed = Committed::new(record(7)).unwrap();
        let whole = serde_json::to_string(&ServerFrame::Event(Box::new(record(7)))).unwrap();
        assert_eq!(committed.frame().as_str(), whole);
    }

    /// Nanoseconds of CPU one event costs the fan-out: the frame spelled
    /// once, one send, then every subscriber takes the text it writes.
    fn fanout_ns_per_event(sockets: usize) -> u128 {
        const EVENTS: u64 = 4096;
        let (events, _) = broadcast::channel::<Committed>(64);
        let mut subscribers: Vec<_> = (0..sockets).map(|_| events.subscribe()).collect();
        let prepared: Vec<EventRecord> = (1..=EVENTS).map(record).collect();
        let mut written = 0usize;
        let started = Instant::now();
        for event in prepared {
            events.send(Committed::new(event).unwrap()).unwrap();
            for subscriber in &mut subscribers {
                let text = subscriber.try_recv().unwrap().frame();
                written = written.wrapping_add(text.len());
            }
        }
        let spent = started.elapsed().as_nanos();
        assert!(written > 0);
        spent / u128::from(EVENTS)
    }

    #[test]
    #[ignore = "an instrument: run with --release --run-ignored only"]
    fn instrument_fanout_cpu_per_event() {
        let readings: Vec<(usize, u128)> = [1, 4, 16]
            .into_iter()
            .map(|sockets| (sockets, fanout_ns_per_event(sockets)))
            .collect();
        for (sockets, ns) in &readings {
            println!("fanout: {sockets} sockets, {ns} ns per event");
        }
        let at_one = readings.first().map(|(_, ns)| *ns).unwrap();
        let at_sixteen = readings.last().map(|(_, ns)| *ns).unwrap();
        assert!(
            at_sixteen <= at_one.saturating_mul(2),
            "16 sockets cost {at_sixteen} ns per event against {at_one} ns at one: the record is serialised per socket again"
        );
    }
}
