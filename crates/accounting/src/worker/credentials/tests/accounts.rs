// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One provider with two accounts, driven through the worker a city
//! runs: which account each request went out on, what the ledger says
//! about it, and which account the Session keeps
//! (`crates/kernel/Spec.lean` §8-86, `crates/gateway/spec/Router.lean`
//! D30).
//!
//! The provider answers each request by the key it carries, so what a
//! test reads off the wire is the account the city chose, and the
//! ledger's `model_called` lines must name the same accounts in the same
//! order.

mod affinity;
mod forgetting;
mod readback;
mod recovery;

use kernel::account_recovery::AccountRetries;
use kernel::{Address, IdemKey, RunId, Seq};

use crate::worker::fixture::*;
use crate::worker::*;

/// The first account's key, as the vault holds it.
const ALPHA: &str = "sk-alpha-0001";
/// The second account's key.
const BRAVO: &str = "sk-bravo-0002";
/// The room every run of these tests goes to.
const ROOM: &str = "lab/room1";

/// A model answer that ends the run.
fn answer() -> Reply {
    Reply::answered(completion("answered", None))
}

/// A busy provider: the same account answers later.
fn busy() -> Reply {
    Reply::refused(
        429,
        &serde_json::json!({ "code": "rate_limit_exceeded", "type": "requests" }),
    )
}

/// A key the provider does not accept.
fn rejected() -> Reply {
    Reply::refused(401, &serde_json::json!({ "code": "invalid_api_key" }))
}

/// An account whose quota is used up.
fn spent() -> Reply {
    Reply::refused(
        429,
        &serde_json::json!({ "code": "insufficient_quota", "type": "insufficient_quota" }),
    )
}

/// A provider that never paces a request.
fn unpaced() -> Pace {
    std::sync::Arc::new(|_: &str| {})
}

/// A city with the two accounts filed, in the order `a`, `b`, and the
/// one model chosen, as the Provider page would leave it.
struct Rig {
    dir: tempfile::TempDir,
    worker: RunWorker,
    provider: FakeProvider,
    base_url: String,
    /// Commands sent so far, which keeps every idempotency key distinct.
    sent: u32,
}

impl Rig {
    fn new(alpha: Vec<Reply>, bravo: Vec<Reply>, pace: Pace) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let (base_url, provider) =
            fake_openai_keyed(&["m-1"], vec![(ALPHA, alpha), (BRAVO, bravo)], pace);
        let worker = opened(dir.path());
        let mut rig = Rig {
            dir,
            worker,
            provider,
            base_url,
            sent: 0,
        };
        rig.attach(&["a", "b"], None);
        rig.command(|idem| wire::Command::SelectModel {
            endpoint: wire::ProviderName::parse("house").unwrap(),
            model: "m-1".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            input: None,
            idem,
        });
        rig
    }

    /// Sends one command, under an idempotency key of its own.
    fn command(&mut self, of: impl FnOnce(IdemKey) -> wire::Command) {
        self.sent = self.sent.saturating_add(1);
        let idem = IdemKey::derive(&RunId::CITY, Seq::FIRST, &self.sent.to_be_bytes());
        self.worker.handle(of(idem)).unwrap();
    }

    /// Files the account list in `order`, with the per-account retry
    /// figure the advanced form would send.
    fn attach(&mut self, order: &[&str], retries: Option<AccountRetries>) {
        let accounts = order
            .iter()
            .map(|id| kernel::event::record::ProviderAccount {
                id: kernel::ServerLabel::parse(id).unwrap(),
                reference: Some(kernel::SecretRef::new("fixture", id).unwrap()),
                header: None,
            })
            .collect();
        let base_url = self.base_url.clone();
        self.command(|idem| wire::Command::AttachEndpoint {
            name: wire::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["m-1".to_owned()],
            tuning: wire::EndpointTuning {
                accounts: Some(accounts),
                account_retries: retries,
                ..wire::EndpointTuning::default()
            },
            idem,
        });
    }

    /// One piece of work for the room, run to its end.
    fn dispatch(&mut self) {
        self.command(|idem| wire::Command::Dispatch {
            addr: Address::parse(ROOM).unwrap(),
            task: "answer once".to_owned(),
            goal: "one answer".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem,
            session: None,
            effort: None,
            model: None,
        });
    }

    /// The city closed, its snapshot removed, and opened again, so what
    /// the new worker knows it folded from the ledger alone. The test
    /// vault keeps nothing across, so both keys are filed again, as a
    /// person's keyring would hand them back.
    fn reopened_without_snapshot(self) -> Rig {
        let Rig {
            dir,
            worker,
            provider,
            base_url,
            sent,
        } = self;
        drop(worker);
        let snapshot = kernel::layout::CityLayout::new(dir.path()).snapshot();
        assert!(snapshot.exists(), "the city wrote no snapshot to remove");
        std::fs::remove_dir_all(&snapshot).unwrap();
        let worker = opened(dir.path());
        Rig {
            dir,
            worker,
            provider,
            base_url,
            sent,
        }
    }

    /// The account each model request went out on, read off its
    /// authorization header: `a` or `b`, `both` or `none`.
    fn wire_accounts(&self) -> Vec<&'static str> {
        self.provider
            .exchanges()
            .iter()
            .filter(|request| request.starts_with("POST "))
            .map(|request| {
                let auth = request
                    .lines()
                    .filter(|line| line.to_ascii_lowercase().starts_with("authorization:"))
                    .collect::<Vec<_>>();
                match auth.as_slice() {
                    [line] if *line == format!("authorization: Bearer {ALPHA}") => "a",
                    [line] if *line == format!("authorization: Bearer {BRAVO}") => "b",
                    [_, _, ..] => "both",
                    [] | [_] => "none",
                }
            })
            .collect()
    }

    /// The ledger, one string per line.
    fn ledger(&self) -> Vec<String> {
        runtime::replay::verify_ledger_dir(
            &kernel::layout::CityLayout::new(self.dir.path()).ledger(),
        )
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect()
    }

    /// The ledger's lines of one kind, decoded.
    fn records(&self, kind: &str) -> Vec<serde_json::Value> {
        self.ledger()
            .iter()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|line| line["kind"] == kind)
            .collect()
    }

    /// What the watchdog did after each failure, in order.
    fn watchdog(&self) -> Vec<String> {
        self.records("watchdog_fired")
            .iter()
            .map(
                |line| match (line["data"]["action"].as_str(), line["data"]["to"].as_str()) {
                    (Some(action), Some(to)) => format!("{action} {to}"),
                    (Some(action), None) => action.to_owned(),
                    (None, _) => format!("{line}"),
                },
            )
            .collect()
    }

    /// The account the Session in the room is bound to.
    fn bound(&self) -> Option<String> {
        self.worker
            .credentials
            .book
            .session_account(&Address::parse(ROOM).unwrap(), "house")
            .map(|account| account.as_str().to_owned())
    }

    /// What every test asks of the wire and the ledger together: each
    /// request carried exactly the key of the account its `model_called`
    /// line names, in the same order, and neither key's value is
    /// anywhere in the ledger.
    fn assert_keys_stay_in_the_vault(&self) {
        let named = self
            .records("model_called")
            .iter()
            .map(|line| {
                line["data"]["provider_account"]["account"]
                    .as_str()
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        let sent = self
            .wire_accounts()
            .into_iter()
            .map(|account| Some(account.to_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            sent, named,
            "each request carries the key of the account it records"
        );
        let ledger = self.ledger().join("\n");
        assert!(
            !ledger.contains(ALPHA) && !ledger.contains(BRAVO),
            "a key's value reached the ledger"
        );
    }
}

/// A worker over the city at `root`, with both keys filed.
fn opened(root: &std::path::Path) -> RunWorker {
    let mut worker = RunWorker::new(
        root,
        runtime::diagnostics::Diagnostics::off(),
        crate::worker::fixture::hands(),
    )
    .unwrap();
    worker.read_volume_with(crate::worker::fixture::roomy_volume);
    for (name, key) in [("a", ALPHA), ("b", BRAVO)] {
        worker
            .handle(wire::Command::PutSecret {
                realm: "fixture".to_owned(),
                name: name.to_owned(),
                value: kernel::Sealed::new(Box::new(key.to_owned())),
            })
            .unwrap();
    }
    worker
}
