// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::*;
use kernel::ledger::chain_hash;
use kernel::{EventDraft, EventKind, Payload, RunId, Seq, TimeMs};
use std::fs;

fn draft(kind: EventKind, t: u64) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(t),
        who: "city".to_string(),
        addr: None,
        kind,
        data: Payload::empty(),
        ig: false,
    }
}

/// A line from a newer vocabulary that marks itself ignorable is lawful
/// history wherever it sits. When it is the last line of the segment
/// before the last one, the boundary still reads the chain off it, the
/// same way the tail scan and replay read it.
#[test]
fn a_prior_segment_ending_in_an_ignorable_line_opens() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    ledger
        .append_all(vec![draft(EventKind::CityInitialized, 1)])
        .unwrap();
    let genesis = ledger.read_raw_lines().unwrap().remove(0);
    drop(ledger);
    let future = format!(
        "{{\"v\":1,\"run\":\"00000000-0000-0000-0000-000000000000\",\"seq\":1,\
         \"prev\":\"{}\",\"t\":0,\"who\":\"city\",\"kind\":\"kind_from_the_future\",\
         \"data\":{{}},\"ig\":true}}\n",
        chain_hash(&genesis)
    );
    let first = dir.path().join(segment_file_name(Seq::FIRST));
    let mut bytes = fs::read(&first).unwrap();
    bytes.extend_from_slice(future.as_bytes());
    fs::write(&first, &bytes).unwrap();

    let (mut reopened, _) = JsonlLedger::open(dir.path(), TimeMs::new(2)).unwrap();
    reopened.set_roll_bytes_for_test(1);
    reopened
        .append_all(vec![draft(EventKind::RunStarted, 2)])
        .unwrap();
    drop(reopened);
    assert!(
        dir.path().join(segment_file_name(Seq::new(2))).exists(),
        "the fixture must put the ignorable line at the end of the prior segment"
    );

    let opened = JsonlLedger::open(dir.path(), TimeMs::new(3))
        .map(|(ledger, _)| ledger.position())
        .map_err(|refused| refused.to_string());
    assert_eq!(
        opened,
        Ok(Seq::new(3)),
        "the prior segment's ignorable last line refused the open"
    );
}
