// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod affinity;
mod forgetting;
mod readback;
mod recovery;

use kernel::account_recovery::AccountRetries;
use kernel::{Address, IdemKey, RunId, Seq};

use crate::worker::fixture::*;
use crate::worker::*;

const ALPHA: &str = "sk-alpha-0001";

const BRAVO: &str = "sk-bravo-0002";

const ROOM: &str = "lab/room1";

fn answer() -> Reply {
    Reply::answered(completion("answered", None))
}

fn busy() -> Reply {
    Reply::refused(
        429,
        &serde_json::json!({ "code": "rate_limit_exceeded", "type": "requests" }),
    )
}

fn rejected() -> Reply {
    Reply::refused(401, &serde_json::json!({ "code": "invalid_api_key" }))
}

fn spent() -> Reply {
    Reply::refused(
        429,
        &serde_json::json!({ "code": "insufficient_quota", "type": "insufficient_quota" }),
    )
}

fn unpaced() -> Pace {
    std::sync::Arc::new(|_: &str| {})
}

struct Rig {
    dir: tempfile::TempDir,
    worker: RunWorker,
    provider: FakeProvider,
    base_url: String,

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

    fn command(&mut self, of: impl FnOnce(IdemKey) -> wire::Command) {
        self.sent = self.sent.saturating_add(1);
        let idem = IdemKey::derive(&RunId::CITY, Seq::FIRST, &self.sent.to_be_bytes());
        self.worker.handle(of(idem)).unwrap();
    }

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
            named: Vec::new(),
            tuning: wire::EndpointTuning {
                accounts: Some(accounts),
                account_retries: retries,
                ..wire::EndpointTuning::default()
            },
            idem,
        });
    }

    fn dispatch(&mut self) {
        self.command(|idem| wire::Command::Dispatch {
            addr: Address::parse(ROOM).unwrap(),
            task: "answer once".to_owned(),
            goal: "one answer".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem,
            session: None,
            resume: None,
            images: Vec::new(),
            effort: None,
            model: None,
        });
    }

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

    fn records(&self, kind: &str) -> Vec<serde_json::Value> {
        self.ledger()
            .iter()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|line| line["kind"] == kind)
            .collect()
    }

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

    fn bound(&self) -> Option<String> {
        self.worker
            .credentials
            .book
            .session_account(&Address::parse(ROOM).unwrap(), "house")
            .map(|account| account.as_str().to_owned())
    }

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
