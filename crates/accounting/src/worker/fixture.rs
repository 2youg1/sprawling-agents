// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

mod rules;

pub(super) use rules::{lay_rules, ordinary_rules, shut_rules};

mod hands;

pub(super) use hands::{LeapingClock, WallClock, monotonic, roomy_volume};
pub(crate) use hands::{hands, init_city};

#[cfg(test)]
mod provider;

// The provider the tests below talk to, re-exported so a test names
// one fixture module and not two.
#[cfg(test)]
pub(super) use provider::{
    FirstChat, Pace, completion, completion_with, fake_openai, fake_openai_paced,
    fake_openai_routed, fake_openai_routed_with,
};

/// A worker with one endpoint attached and one model chosen, exactly
/// as the settings page would leave it.
pub(super) fn worker_with_provider(
    city_root: &Path,
    base_url: &str,
    model: &str,
) -> Result<RunWorker, AxError> {
    let worker = RunWorker::new(
        city_root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )?;
    attach_provider(worker, base_url, model)
}

/// The same endpoint and model, attached to a worker somebody else
/// opened - over a ledger on another `Vfs`, say. The worker reads a
/// roomy volume rather than the host's disk, so a dispatch test does not
/// turn into `BackpressureShed` on a host below the free-space floor.
pub(super) fn attach_provider(
    mut worker: RunWorker,
    base_url: &str,
    model: &str,
) -> Result<RunWorker, AxError> {
    worker.read_volume_with(roomy_volume);
    worker.handle(wire::Command::AttachEndpoint {
        name: wire::ProviderName::parse("house").unwrap(),
        base_url: base_url.to_owned(),
        dialect: kernel::DialectKind::OpenAi,
        secret: None,
        auth_header: None,
        admit: Vec::new(),
        tuning: wire::EndpointTuning::default(),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
    })?;
    worker.handle(wire::Command::SelectModel {
        endpoint: wire::ProviderName::parse("house").unwrap(),
        model: model.to_owned(),
        tag: kernel::ModelTag::Main,
        context_tokens: kernel::Window::new(32_768),
        max_output_tokens: kernel::Ceiling::new(4_096),
        idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
    })?;
    Ok(worker)
}

/// A worker over a ledger that loses the first append carrying `cut`,
/// and loses nothing when `cut` is `None`.
pub(super) fn worker_over_faults(root: &Path, cut: Option<&'static str>) -> RunWorker {
    let fs = storage::FaultFs::new(storage::FaultPlan {
        cut_at_op: None,
        cut_on_write: cut,
        torn_tail: storage::TornTail::None,
    });
    let opened = storage::JsonlLedger::open_faulty(
        fs,
        &kernel::layout::CityLayout::new(root).ledger(),
        crate::Clock::now(&WallClock).unwrap(),
    )
    .unwrap();
    RunWorker::over(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
        opened,
    )
    .unwrap()
}

/// Every line the nodes of a workshop wrote, in order, with the kind of
/// each - because a handdown that does not come back is a race in what
/// the city asked for, and the kind of the line that ended a node is what
/// says why it ended.
///
/// The `lab/` prefix is the fixture's own: it is the building every
/// workshop scenario in this module raises, and a diagnostic that guessed
/// wider would print the whole history.
pub(super) fn node_lines(city_root: &Path) -> Vec<String> {
    runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(city_root).ledger())
        .map(|verified| {
            verified
                .raw_lines()
                .iter()
                .filter_map(|line| serde_json::from_slice::<serde_json::Value>(line).ok())
                .filter(|line| {
                    line["who"]
                        .as_str()
                        .is_some_and(|who| who.starts_with("lab/"))
                })
                .map(|line| {
                    format!(
                        "seq {} {} {} run {} {:?}",
                        line["seq"], line["who"], line["kind"], line["run"], line["data"]
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Breaks the history's hash chain at its second line, leaving every
/// line a well-formed record: that line's `prev` no longer names the
/// genesis line.
///
/// A reader that verifies the history refuses this ledger; a reader that
/// reads only the lines it was asked about does not notice. That
/// difference, rather than a timing, is what shows a query stayed off the
/// verify path.
pub(super) fn break_the_chain_after_genesis(city_root: &Path) {
    let segment = storage::ledger_segments_at(&kernel::layout::CityLayout::new(city_root).ledger())
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let mut bytes = std::fs::read(&segment).unwrap();
    let key = b"\"prev\":\"";
    let second = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .and_then(|end| end.checked_add(1))
        .unwrap();
    let digit = bytes[second..]
        .windows(key.len())
        .position(|window| window == key)
        .and_then(|at| second.checked_add(at)?.checked_add(key.len()))
        .unwrap();
    bytes[digit] = if bytes[digit] == b'0' { b'1' } else { b'0' };
    std::fs::write(&segment, bytes).unwrap();
    assert!(
        runtime::replay::verify_ledger_dir(&kernel::layout::CityLayout::new(city_root).ledger())
            .is_err(),
        "the broken chain is refused by a verify"
    );
}

/// One completion that calls a named tool with the given arguments.
/// Separate from `completion` because that helper hard-codes the edit
/// tool's shape, and a tool面 test is about a different tool.
pub(super) fn tool_completion(text: &str, id: &str, tool: &str, args: serde_json::Value) -> String {
    serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": text,
                "tool_calls": [{
                    "id": id,
                    "type": "function",
                    "function": { "name": tool, "arguments": args.to_string() },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 5 },
    })
    .to_string()
}

pub(super) const PLAN_TWO_FREE_ROWS: &str = concat!(
    "| # | Item | Weight | Needs | Status | Evidence |\n",
    "|---|---|---|---|---|---|\n",
    "| 1 | wire the kiln | 1 |  | Not started |  |\n",
    "| 2 | glaze tests | 1 |  | Not started |  |\n",
);

/// Three rows nothing blocks, which is a ready set of three: what a
/// pursuit takes the whole of rather than one node at a time
/// (sprawling-SPEC 8-46-4).
pub(super) const PLAN_THREE_FREE_ROWS: &str = concat!(
    "| # | Item | Weight | Needs | Status | Evidence |\n",
    "|---|---|---|---|---|---|\n",
    "| 1 | wire the kiln | 1 |  | Not started |  |\n",
    "| 2 | glaze tests | 1 |  | Not started |  |\n",
    "| 3 | stack the shelves | 1 |  | Not started |  |\n",
);

pub(super) const PLAN_ONE_FREE_ROW: &str = concat!(
    "| # | Item | Weight | Needs | Status | Evidence |\n",
    "|---|---|---|---|---|---|\n",
    "| 1 | wire the kiln | 1 |  | Not started |  |\n",
);

/// A control surface that listens for interrupts and nothing else.
///
/// The four sinks are one value and one injection, so a test that
/// cares about steers says what it does not listen for rather than
/// reaching for a setter of its own.
pub(super) fn only_interrupts(
    source: std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>,
) -> Serving {
    Serving {
        deltas: std::sync::Arc::new(|_delta| {}),
        outputs: std::sync::Arc::new(|_piece| {}),
        machine: std::sync::Arc::new(|_found| {}),
        interrupts: source,
    }
}

/// **The product defect this knob was built to catch.** One chat
/// request lands and the connection closes before a byte comes back,
/// and both handdown runs still arrive: a failure that completed no
/// exchange is retriable, so the default `UntilHalted` asks again.
/// While no provider failure was ever retriable, this one drop froze
/// the run and the parent joined nothing.
#[test]
fn a_dropped_call_is_asked_again_and_both_handdowns_still_come_back() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let graph = serde_json::json!({ "op": "lay_out", "nodes": [
        { "room": "lab/writer", "goal": "write it up", "done_check": "the page exists",
          "stop": "when the page exists", "depends_on": ["lab/reader"] },
        { "room": "lab/reader", "goal": "read the meter", "stop": "when it is written down",
          "done_check": "a number is written down" },
    ] });
    let (base_url, _provider) = fake_openai_routed_with(
        &["m-local"],
        vec![
            ("a number is written down", vec![completion("done", None)]),
            ("the page exists", vec![completion("done", None)]),
        ],
        vec![
            completion_with("splitting it up", "workshop", "tu_1", graph.clone()),
            completion("waiting on a person", None),
            completion_with("splitting it up", "workshop", "tu_2", graph),
            completion("done", None),
        ],
        FirstChat::Dropped,
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(wire::Command::CreateBuilding {
            addr: Address::parse("lab").unwrap(),
            template: wire::TemplateName::parse("minimal").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"create"),
        })
        .unwrap();
    let room = Address::parse("lab/room1").unwrap();
    worker
        .handle(wire::Command::Dispatch {
            addr: room.clone(),
            task: "get it measured and written up".to_owned(),
            goal: "a page with a number in it, then stop".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
            model: None,
        })
        .unwrap();
    let joined = worker
        .collaborating
        .joins
        .get(&room)
        .map_or(0, |j| j.artifacts().count());
    // Why a missing handback is worth a paragraph: the failure is a
    // race in what the provider was asked, so the answer is which
    // run got what, not which line the cell says is false.
    let said: Vec<String> = _provider
        .bodies()
        .iter()
        .map(|body| {
            let room_of = ["a number is written down", "the page exists"]
                .into_iter()
                .filter(|mark| body.contains(mark))
                .collect::<Vec<_>>();
            format!("{room_of:?}")
        })
        .collect();
    let ends = node_lines(dir.path());
    assert_eq!(
        joined,
        2,
        "a dropped connection cost a handdown: the call was never asked again;
         the requests named {said:?};
{}",
        ends.join("\n")
    );
}
