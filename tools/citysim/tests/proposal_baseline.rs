// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A proposal made on a version its document has since left is refused
//! when the person decides it, and the document keeps the bytes it
//! moved to (citysim D22; documents D17, D35; `crates/accounting/Spec.lean` §8-30).
//!
//! The city is the product's: a worker built from scripted hands - a
//! counted clock, a vault in memory - and a scripted model. The run's
//! `proposal` tool, the relay it writes through, the governance fold and
//! the person's decision are the ones a served city runs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use accounting::worker::RunWorker;
use accounting::worker::hands::Hands;
use citysim::{ScriptModel, concluding};
use kernel::event::record::{SliceVerdict, Verdict};
use kernel::layout::CityLayout;
use kernel::{
    Address, AxCode, AxError, ContentBlock, IdemKey, ModelReturn, Payload, RunId, Seq, TimeMs,
    ToolCall, ToolName,
};
use serde_json::json;

/// The room the run works in, and the document it proposes a change to.
const ROOM: &str = "hall/mayor";
const DOC: &str = "hall/mayor/draft.md";

const DRAFT: &str = "# Draft\n\nThe kiln is fired at noon. It cools overnight.\n";

/// What the person's own editor makes of the draft while the card waits.
const MOVED: &str = "# Draft\n\nThe kiln is fired at night. It cools overnight.\n";

/// A clock that counts the times it is read, from a fixed start.
struct Counted(AtomicU64);

impl accounting::Clock for Counted {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(
            1_790_000_000_000_u64.saturating_add(self.0.fetch_add(1, Ordering::Relaxed)),
        ))
    }
}

/// The product's hands, with the clock counted and the vault in memory.
fn hands(clock: &Arc<dyn accounting::Clock + Send + Sync>) -> Hands {
    Hands {
        clock: Arc::clone(clock),
        ..sprawling::assembly::hands(gateway::Custodian::in_memory())
    }
}

/// Hands the script to the first model the worker asks for, and an
/// empty one to any later ask.
struct Scripted(Mutex<Option<ScriptModel>>);

impl accounting::ModelFactory for Scripted {
    fn build(
        &self,
        _chosen: &gateway::Chosen<'_>,
        _redemption: gateway::Redemption,
    ) -> Result<Box<dyn kernel::Model + Send>, AxError> {
        Ok(Box::new(
            self.0
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(ScriptModel::silent),
        ))
    }
}

/// One turn that offers the change, then one that says it is done.
fn script() -> ScriptModel {
    let call = ToolCall {
        id: "call-1".to_owned(),
        name: ToolName::parse("proposal").unwrap(),
        args: Payload::new(
            json!({
                "action": "offer",
                "path": DOC,
                "old": "It cools overnight.",
                "new": "It cools by dawn.",
            })
            .as_object()
            .cloned()
            .unwrap(),
        )
        .unwrap(),
    };
    let offering = ModelReturn::bare(
        kernel::model::message_payload(&[ContentBlock::ToolUse {
            id: call.id.clone(),
            name: call.name.clone(),
            input: call.args.clone(),
        }])
        .unwrap(),
        vec![call],
    );
    ScriptModel::new(vec![offering, concluding("the card is offered").unwrap()])
}

fn idem(what: &[u8]) -> IdemKey {
    IdemKey::derive(&RunId::CITY, Seq::FIRST, what)
}

/// A founded city whose main model sits behind a loopback port nothing
/// listens on, so only the script answers.
fn city(dir: &Path) -> RunWorker {
    let clock: Arc<dyn accounting::Clock + Send + Sync> = Arc::new(Counted(AtomicU64::new(0)));
    accounting::worker::genesis::form(
        dir,
        accounting::worker::genesis::Adopt::Nothing,
        hands(&clock),
    )
    .unwrap();
    let mut worker = RunWorker::new(dir, runtime::diagnostics::Diagnostics::off(), hands(&clock))
        .unwrap()
        .with_models(Box::new(Scripted(Mutex::new(Some(script())))));
    let refusing = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}/v1", refusing.local_addr().unwrap());
    drop(refusing);
    let endpoint = wire::ProviderName::parse("dead").unwrap();
    worker
        .handle(wire::Command::AttachEndpoint {
            name: endpoint.clone(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["scripted".to_owned()],
            tuning: wire::EndpointTuning {
                timeout_ms: Some(2_000),
                request_max_retries: Some(0),
                ..wire::EndpointTuning::default()
            },
            idem: idem(b"attach"),
        })
        .unwrap();
    worker
        .handle(wire::Command::SelectModel {
            endpoint,
            model: "scripted".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(131_072),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem: idem(b"select"),
        })
        .unwrap();
    worker
}

fn path_of(dir: &Path, addr: &str) -> PathBuf {
    addr.split('/')
        .fold(dir.to_path_buf(), |at, part| at.join(part))
}

/// The cards open on the draft.
fn open_cards(dir: &Path) -> Vec<wire::ProposalCard> {
    match accounting::views::ask(dir, &wire::Query::Proposals(Address::parse(DOC).unwrap())) {
        Ok(wire::Answer::Proposals(cards)) => cards.open,
        other => panic!("the cards answered {other:?}"),
    }
}

/// Every changed sentence of `card` accepted.
fn accept_all(card: &wire::ProposalCard) -> Vec<SliceVerdict> {
    (0_u32..)
        .zip(&card.slices)
        .filter(|(_, slice)| serde_json::to_value(slice).unwrap()["kind"] != "same")
        .map(|(slice, _)| SliceVerdict {
            slice,
            verdict: Verdict::Accept,
        })
        .collect()
}

#[test]
fn a_proposal_made_on_a_version_the_document_left_is_refused_when_decided() {
    let dir = tempfile::tempdir().unwrap();
    let mut worker = city(dir.path());
    let room = Address::parse(ROOM).unwrap();
    let urbanite = CityLayout::new(dir.path()).urbanite(&room);
    std::fs::create_dir_all(urbanite.parent().unwrap()).unwrap();
    std::fs::write(&urbanite, "# URBANITE.md\n\nKeeps the drafts.\n").unwrap();
    let draft = path_of(dir.path(), DOC);
    std::fs::write(&draft, DRAFT).unwrap();

    let dispatched = worker.handle(wire::Command::Dispatch {
        addr: room,
        task: "Offer the change to the draft.".to_owned(),
        goal: "the card is offered".to_owned(),
        policy: kernel::RunPolicy::of(kernel::Mode::Work),
        idem: idem(b"dispatch"),
        session: None,
        effort: None,
        model: None,
    });
    let offered = open_cards(dir.path());
    assert_eq!(
        offered.len(),
        1,
        "one card is open after the run; the dispatch answered {dispatched:?}"
    );

    // The person's own editor moves the draft while the card waits.
    std::fs::write(&draft, MOVED).unwrap();
    let decided = worker.handle(wire::Command::DecideProposals(wire::ProposalDecisions {
        doc: Address::parse(DOC).unwrap(),
        decisions: vec![wire::ProposalDecision {
            proposal: offered[0].id,
            verdicts: accept_all(&offered[0]),
        }],
        idem: idem(b"decide"),
    }));

    assert_eq!(
        decided.map_err(|err| *err.code()),
        Err(AxCode::VersionConflict)
    );
    assert_eq!(std::fs::read_to_string(&draft).unwrap(), MOVED);
    assert_eq!(open_cards(dir.path()), offered);
}
