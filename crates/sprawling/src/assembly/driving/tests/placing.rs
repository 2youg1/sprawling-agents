// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::cell::RefCell;
use std::sync::{Arc, Condvar, Mutex};

use kernel::{AxError, Effect, Payload, RunId, TimeMs, Tool, ToolCall, ToolName, ToolOutcome};
use runtime::{Admitted, ConcurrentInvoke};

use super::super::Sieving;
use super::super::placing::Placing;

const READS: u32 = 3;

/// Long enough that a thread the scheduler delays under a loaded build
/// still arrives; reads run one after another wait it out and fail.
const ARRIVAL_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Where the reads of one wave meet: each read waits until every read
/// has started, then notes how many had started while it still ran.
struct Meeting {
    started: Mutex<u32>,
    arrived: Condvar,
    fewest_seen: Mutex<u32>,
}

/// A read-only tool; with a meeting, each call waits at it.
struct MeetingRead {
    meta: kernel::ToolMeta,
    meeting: Option<Arc<Meeting>>,
}

impl Tool for MeetingRead {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if let Some(meeting) = &self.meeting {
            let mut started = meeting.started.lock().unwrap();
            *started = started.saturating_add(1);
            meeting.arrived.notify_all();
            let (started, _) = meeting
                .arrived
                .wait_timeout_while(started, ARRIVAL_WAIT, |seen| *seen < READS)
                .unwrap();
            let mut fewest = meeting.fewest_seen.lock().unwrap();
            *fewest = (*fewest).min(*started);
        }
        Ok(ToolOutcome {
            result: Payload::of(&serde_json::json!({ "read": call.id })).unwrap(),
            attachments: Vec::new(),
        })
    }
}

fn placing<'f>(
    meeting: Option<Arc<Meeting>>,
    dir: &std::path::Path,
    fenced: &'f RefCell<Vec<String>>,
) -> Placing<'f> {
    let domain = kernel::WriteDomain::new(vec![kernel::Address::parse("lab").unwrap()]).unwrap();
    let mut bench = runtime::bench::ToolBench::new(domain);
    bench
        .register(Box::new(MeetingRead {
            meta: kernel::ToolMeta {
                name: ToolName::parse("read").unwrap(),
                disclosure: "reads a file".to_owned(),
                params: Payload::empty(),
                effect: Effect::Read,
                cost_tier: kernel::CostTier::Free,
                timeout: None,
                render: kernel::RenderIntent::Generic,
                temporal: kernel::Temporal::Timeless,
            },
            meeting,
        }))
        .unwrap();
    let sieving = Sieving {
        cas: memory::Cas::open(&dir.join("cas")).unwrap(),
        city_root: dir.to_path_buf(),
        room: kernel::Address::parse("lab").unwrap(),
        table: runtime::FilterTable::builtin(),
        history: runtime::SieveHistory::default(),
    };
    Placing::new(bench, sieving, RunId::CITY, fenced)
}

fn reads() -> Vec<ToolCall> {
    ["r1", "r2", "r3"]
        .into_iter()
        .map(|id| ToolCall {
            id: id.to_owned(),
            name: ToolName::parse("read").unwrap(),
            args: Payload::of(&serde_json::json!({ "path": id })).unwrap(),
        })
        .collect()
}

/// The served city's tool face names its reads as reads, so a wave runs
/// them at once, and the answers, the fence list and the command counts
/// it leaves are those the same calls leave one after another.
#[test]
fn a_served_citys_reads_run_at_once_and_leave_what_they_leave_in_turn() {
    let dir = tempfile::tempdir().unwrap();
    let calls = reads();
    let t = TimeMs::new(1);

    let serial_fenced = RefCell::new(Vec::new());
    let mut one_by_one = placing(None, dir.path(), &serial_fenced);
    let serial: Vec<ToolOutcome> = calls
        .iter()
        .map(|call| match one_by_one.admit(call, t) {
            Admitted::Cleared(ticket) => {
                let ran = one_by_one.tool(&ticket).unwrap().invoke(call);
                one_by_one.account(call, ticket, ran).unwrap()
            }
            Admitted::Answered(answered) => answered.unwrap(),
        })
        .collect();

    let meeting = Arc::new(Meeting {
        started: Mutex::new(0),
        arrived: Condvar::new(),
        fewest_seen: Mutex::new(u32::MAX),
    });
    let fenced = RefCell::new(Vec::new());
    let mut lane = placing(Some(Arc::clone(&meeting)), dir.path(), &fenced);
    assert!(
        calls
            .iter()
            .all(|call| lane.effect_of(call) == Some(Effect::Read)),
        "the lane does not name its reads as reads, so every wave runs serially"
    );
    let tickets: Vec<_> = calls
        .iter()
        .map(|call| match lane.admit(call, t) {
            Admitted::Cleared(ticket) => ticket,
            Admitted::Answered(answered) => panic!("answered at admission: {answered:?}"),
        })
        .collect();
    let tools: Vec<&dyn Tool> = tickets
        .iter()
        .map(|ticket| lane.tool(ticket).unwrap())
        .collect();
    let ran: Vec<Result<ToolOutcome, AxError>> = std::thread::scope(|scope| {
        let running: Vec<_> = tools
            .iter()
            .zip(&calls)
            .map(|(tool, call)| scope.spawn(move || tool.invoke(call)))
            .collect();
        running.into_iter().map(|one| one.join().unwrap()).collect()
    });
    let at_once: Vec<ToolOutcome> = calls
        .iter()
        .zip(tickets)
        .zip(ran)
        .map(|((call, ticket), ran)| lane.account(call, ticket, ran).unwrap())
        .collect();

    assert_eq!(*meeting.fewest_seen.lock().unwrap(), READS);
    assert_eq!((at_once, lane.ran()), (serial, one_by_one.ran()));
    drop((lane, one_by_one));
    assert_eq!(fenced.into_inner(), serial_fenced.into_inner());
}
