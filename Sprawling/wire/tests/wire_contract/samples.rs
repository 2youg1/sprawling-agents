// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, Sealed, Seq};

use wire::{Command, Mode, ProviderName};

pub(super) fn sample_of_every_command() -> Vec<Command> {
    let addr = Address::parse("acme/floor1").unwrap();
    let run = kernel::RunId::from_bytes([7u8; 16]);
    let idem = kernel::IdemKey::derive(&run, Seq::new(1), b"sample");
    vec![
        Command::Wake {
            source: "github".to_owned(),
            subject: "pull request opened".to_owned(),
            body: "someone would like a change reviewed".to_owned(),
            idem,
        },
        Command::Dispatch {
            addr: addr.clone(),
            task: "ship it".to_owned(),
            goal: "the tests pass".to_owned(),
            policy: wire::RunPolicy::of(Mode::Work),
            idem,
            session: Some(kernel::SessionName::parse("ship it").unwrap()),
            effort: Some(kernel::Effort::High),
            model: None,
        },
        Command::PutIdentity {
            card: wire::IdentityCard::Mayor {
                name: Some("Cat".to_owned()),
            },
            base: String::new(),
            idem,
        },
        Command::PutRules(wire::RulesWrite {
            building: Address::parse("lab").unwrap(),
            base: String::new(),
            body: "confidential = false".to_owned(),
            idem,
        }),
        Command::SetWorkspaceChoice(wire::WorkspaceChoiceWrite {
            building: Address::parse("lab").unwrap(),
            base: String::new(),
            endpoint: "house".to_owned(),
            model: "ordinary".to_owned(),
            effort: kernel::Effort::Medium,
            idem,
        }),
        Command::RestoreFile {
            at: Address::parse("lab/notes.md").unwrap(),
            point: kernel::GitOid::parse("8ea61adc1c8339d02a5c665ba8280a85151ee549").unwrap(),
            idem,
        },
        Command::PutGuide {
            progress: wire::GuideProgress {
                at: Some(wire::GuideStep::Texts),
                dependencies: Some(wire::GuideMark::Skipped),
                ..wire::GuideProgress::default()
            },
            idem,
        },
        Command::ConfigureCity(wire::CitySettings {
            keep_warm: Some(kernel::KeepWarm::FiveMinute),
            effort: None,
            search: Some(kernel::config::SearchConfiguration::Off),
            catalog: None,
            skill_shelves: None,
            idem,
        }),
        Command::PutRange(wire::RangeWrite {
            doc: Address::parse("lab/notes.md").unwrap(),
            baseline: kernel::B3Hash::digest(b""),
            edits: vec![documents::TextEdit {
                span: documents::Span::new(0, 0).unwrap(),
                text: "first".to_owned(),
            }],
            idem,
        }),
        Command::DecideProposals(wire::ProposalDecisions {
            doc: Address::parse("lab/notes.md").unwrap(),
            decisions: vec![wire::ProposalDecision {
                proposal: kernel::B3Hash::digest(b"card"),
                verdicts: vec![kernel::event::record::SliceVerdict {
                    slice: 1,
                    verdict: kernel::event::record::Verdict::Accept,
                }],
            }],
            idem,
        }),
        Command::PutDocument {
            which: wire::GovernedDocument::Mayor,
            base: String::new(),
            body: "# who the Mayor is
"
            .to_owned(),
            idem,
        },
        Command::PutSpine {
            building: Address::parse("lab").unwrap(),
            which: wire::SpineDocument::Memo,
            base: String::new(),
            body: "# Memo — lab
"
            .to_owned(),
            idem,
        },
        Command::ConfigureBuilding(wire::BuildingSettings {
            addr: Address::parse("lab").unwrap(),
            sandbox: Some(kernel::SandboxLimits::default()),
            mcp: Some(Vec::new()),
            desktop: Some(
                "[[window]]
title = \"a window\"
"
                .to_owned(),
            ),
            context_second_threshold: None,
            catalog: Some(kernel::config::ExposureTable {
                tools: [("exec".to_owned(), kernel::config::Exposure::Dormant)].into(),
                namespaces: [("apps".to_owned(), kernel::config::Exposure::Off)].into(),
                skills: std::collections::BTreeMap::new(),
            }),
            mcp_references: None,
            skill_pins: None,
            reading_room: None,
            idem,
        }),
        Command::ProbeEndpoint {
            name: ProviderName::parse("house").unwrap(),
            base_url: "https://api.example.test/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            secret: Some("secret:house/key".to_owned()),
            auth_header: None,
            tuning: wire::EndpointTuning::default(),
            idem,
        },
        Command::AttachEndpoint {
            name: ProviderName::parse("house").unwrap(),
            base_url: "https://api.example.test/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            secret: Some("secret:house/key".to_owned()),
            auth_header: None,
            admit: vec!["gpt-x".to_owned()],
            named: Vec::new(),
            tuning: wire::EndpointTuning {
                accounts: None,
                label: Some("House".to_owned()),
                timeout_ms: Some(60_000),
                request_max_retries: Some(4),
                stream_idle_timeout_ms: Some(300_000),
                headers: vec![wire::HeaderPair {
                    name: "x-tenant".to_owned(),
                    value: "secret:house/tenant".to_owned(),
                }],
                overrides: vec![wire::BodyOverride {
                    pointer: "/reasoning/effort".to_owned(),
                    value: "high".to_owned(),
                }],
                proxying: Some(kernel::Proxying::Always),
                max_in_flight: None,
                account_retries: Some(kernel::account_recovery::AccountRetries::One),
                generation: Vec::new(),
            },
            idem,
        },
        Command::SelectModel {
            endpoint: ProviderName::parse("house").unwrap(),
            model: "m-large".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: wire::Window::new(128_000),
            max_output_tokens: kernel::Ceiling::new(8_192),
            input: None,
            idem,
        },
        Command::OpenSession {
            addr: addr.clone(),
            carry: wire::Carry::Handoff,
            from: Some(kernel::Origin {
                run,
                at_seq: Seq::new(3),
            }),
            idem,
        },
        Command::CreateBuilding {
            addr: addr.clone(),
            template: wire::TemplateName::parse("workshop").unwrap(),
            rules: None,
            source: None,
            idem,
        },
        Command::RemoveBuilding {
            addr: addr.clone(),
            idem,
        },
        Command::PutSecret {
            realm: "anthropic".to_owned(),
            name: "api".to_owned(),
            value: Sealed::new(Box::new("sk-not-a-real-key".to_owned())),
        },
        Command::Steer {
            run,
            text: "try the other branch".to_owned(),
            idem,
        },
        Command::Cancel { run, idem },
        Command::Halt {
            scope: wire::HaltScope::City,
            idem,
        },
        Command::Release {
            scope: wire::HaltScope::City,
            idem,
        },
        Command::BatchByBuilding {
            addr: addr.clone(),
            idem,
        },
        Command::Approve {
            item: kernel::ApprovalId::new("ap-1").unwrap(),
            verdict: kernel::Ruling::Allow,
            idem,
        },
        Command::HandOff {
            item: kernel::ApprovalId::new("ap-1").unwrap(),
            to: kernel::ResidentId::new("clerk").unwrap(),
            idem,
        },
        Command::PutPreferences {
            patch: wire::PreferencePatch::Lang(wire::Lang::Zh),
            idem,
        },
        Command::PutShelved {
            shelf: wire::Shelf::Library,
            name: "reviewing/first-pass.md".to_owned(),
            text: "# read the SPEC first\n".to_owned(),
            idem,
        },
        Command::SetAutonomy {
            scope: wire::HaltScope::City,
            autonomy: kernel::Autonomy::Owner,
            idem,
        },
        Command::Pursue {
            addr,
            step: wire::PursuitStep::Set {
                goal: "raise the east wing".to_owned(),
            },
            idem,
        },
        Command::Auth {
            token: "pair-me-0123456789".to_owned(),
        },
        Command::Reveal {
            at: Address::parse("hall/JOB.md").unwrap(),
            idem,
        },
        Command::RestoreDiscard {
            restoration: kernel::Restoration::Rebuildable {
                reason: "regenerated by the build".to_owned(),
            },
            idem,
        },
        Command::DoctorInstall {
            item: "cargo-nextest".to_owned(),
            idem,
        },
        Command::DoctorRefresh { idem },
        Command::ConnectToolkit {
            toolkit: wire::ToolkitSlug::parse("github").unwrap(),
            idem,
        },
        Command::NameSession(wire::SessionNaming {
            room: Address::parse("acme/floor1").unwrap(),
            began: Seq::new(3),
            name: "east wing review".to_owned(),
            idem,
        }),
        Command::ChangeRunPolicy(wire::PolicyChange {
            room: Address::parse("acme/floor1").unwrap(),
            policy: wire::RunPolicy::of(Mode::Work),
            idem,
        }),
        Command::OpenRemoteDoor(wire::DoorOpening {
            lasting_ms: 60 * 60 * 1000,
            idem,
        }),
        Command::ReplaceCityKey(wire::DoorStep { idem }),
        Command::ConfirmRemoteDoor(wire::DoorAnswer {
            code: "abcd-efgh".to_owned(),
            idem,
        }),
        Command::CloseRemoteDoor(wire::DoorStep { idem }),
        Command::PrivacyOperation(wire::PrivacyRequest {
            action: wire::PrivacyAction::Apply {
                control: wire::PrivacyControl::StartLaunchTracking,
                expected: wire::PrivacyValue::Dword { number: 1 },
            },
            idem,
        }),
        Command::ForgetSecret(wire::SecretForgetting {
            reference: "secret:search/brave.main".to_owned(),
            idem,
        }),
        Command::CloseCity(wire::CityClosing {
            mode: wire::CloseMode::Drain,
            idem,
        }),
        Command::AddAgent(wire::AgentAdding {
            spec_digest: kernel::B3Hash::digest(b"launch"),
            source: wire::AgentSource::Detected,
            seat_here: None,
            idem,
        }),
        Command::AgentLogin(wire::AgentLoginStart {
            agent: "pi".to_owned(),
            method: "terminal".to_owned(),
            idem,
        }),
        Command::ForgetDevice(wire::DeviceForgetting {
            device: wire::DeviceId::new("browser".to_owned()),
            idem,
        }),
        Command::PutPrompt(wire::PromptWrite {
            at: Address::parse("acme/.sprawling/City.md").unwrap(),
            base: kernel::B3Hash::digest(b""),
            step: wire::PromptStep::Own,
            idem,
        }),
    ]
}
