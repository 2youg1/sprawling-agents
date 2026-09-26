// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A committed record and its one wire frame (channels-SPEC.md 8-47).

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::time::Instant;

    use kernel::{B3Hash, EventRecord, Seq, TimeMs};
    use tokio::sync::broadcast;

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

    /// Nanoseconds of CPU one event costs the fan-out: one send, then
    /// every subscriber takes it and holds the text it would write.
    fn fanout_ns_per_event(sockets: usize) -> u128 {
        const EVENTS: u64 = 4096;
        let (events, _) = broadcast::channel::<EventRecord>(64);
        let mut subscribers: Vec<_> = (0..sockets).map(|_| events.subscribe()).collect();
        let prepared: Vec<EventRecord> = (1..=EVENTS).map(record).collect();
        let mut written = 0usize;
        let started = Instant::now();
        for event in prepared {
            events.send(event).unwrap();
            for subscriber in &mut subscribers {
                let received = subscriber.try_recv().unwrap();
                let text = serde_json::to_string(&ServerFrame::Event(Box::new(received))).unwrap();
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
        let at_sixteen = readings.last().map(|(_, ns)| *ns).unwrap();
        assert!(
            at_sixteen <= 1_000,
            "16 sockets cost {at_sixteen} ns per event, over the 1 µs budget"
        );
    }
}
