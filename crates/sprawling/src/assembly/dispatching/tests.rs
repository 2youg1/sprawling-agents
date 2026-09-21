// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// The other half of the same rule: a steer-kind signal slips under
/// the door of the run it reaches, landing at that run's next safe
/// point with the sender's address in front of it. `collab::steer`
/// holds both entrances, and the resident's one had no caller until
/// now.
#[test]
fn a_steer_from_a_resident_lands_in_the_window_as_that_resident() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    city::create_building(
        dir.path(),
        &Address::parse("market").unwrap(),
        city::BuildingTemplate::Minimal,
    )
    .unwrap();
    for who in ["ito", "hana"] {
        let room = dir.path().join("market").join(who);
        std::fs::create_dir_all(&room).unwrap();
        std::fs::write(
            room.join(city::URBANITE_FILE),
            format!("# URBANITE.md\n\nTrades in the market as {who}.\n"),
        )
        .unwrap();
    }

    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "cutting in",
                "tu_1",
                "signal",
                serde_json::json!({
                    "action": "send",
                    "to": "market/hana",
                    "kind": "steer",
                    "text": "drop the glaze order, the kiln comes first",
                }),
            ),
            completion("sent", None),
            tool_completion("where do I stand", "tu_2", "status", serde_json::json!({})),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("market/ito").unwrap(),
            task: "tell hana what matters first".to_owned(),
            goal: "hana knows".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("@market/ito: drop the glaze order"),
        "the steer lands at the end of a tool result, attributed to the resident who sent it"
    );
    assert!(
        !asked.contains("user: drop the glaze order"),
        "only the person's entrance can render as the person"
    );
}

/// A dispatch the city will not take leaves nothing a person can
/// find.
///
/// `prepare_dispatch` opens by saying so - a halted city that laid a job
/// file down would leave a task in a room no run ever opened - and
/// the halt was the only door that held it. The room was opened and
/// the brief was written before the tag was resolved, so a dispatch
/// to a building nobody raised was refused for having no model and
/// left a building, a room and a `JOB.md` behind: files a person can
/// see that no record in the one history mentions.
///
/// The code is asserted beside the disk because the two are separate
/// promises. The caller is owed `E_CONFIG_INVALID` here - nothing is
/// attached, so no tag names a model - and moving the judgement in
/// front of the write must not change which refusal that is.
#[test]
fn a_dispatch_the_city_will_not_take_leaves_no_room_behind() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::off(),
    )
    .unwrap();
    let refused = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("gamma").unwrap(),
            task: "say something".to_owned(),
            goal: "an answer".to_owned(),
            mode: kernel::Mode::Up,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: Some(kernel::SessionName::parse("one").unwrap()),
            effort: None,
        })
        .unwrap_err();

    assert_eq!(
        *refused.code(),
        AxCode::ConfigInvalid,
        "a city with nothing attached refuses a dispatch for having no model: {refused}"
    );
    assert!(
        !dir.path().join("gamma").exists(),
        "a refused dispatch raised a building nobody asked for"
    );
}

/// A dispatch that never says when to stop is a conversation, and the
/// prefix says so instead of handing over a form with its one
/// irreplaceable field blank.
#[test]
fn a_dispatch_with_no_goal_leaves_no_job_file_and_says_the_person_is_here() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let room = Address::parse("lab/room1").unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: room.clone(),
            task: "what do you make of this".to_owned(),
            goal: String::new(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"talk"),
            session: None,
            effort: None,
        })
        .unwrap();

    assert!(
        !city::job_path(dir.path(), &room).exists(),
        "a conversation writes no task file"
    );
    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("working with the person directly"),
        "the prefix states the situation: {asked}"
    );
    assert!(
        !asked.contains("Task: what do you make of this"),
        "the person's line goes out as they wrote it, not as a form"
    );
}

/// A confidential building's task text is not sent off this machine to
/// be given a room name.
///
/// The main model is chosen on this machine, so the city takes the
/// work; the digest model that names the room is not, and the address
/// is a bare building — the shape the welcome flow teaches, and the one
/// that asks for a name. The refusal must come from the book, under the
/// building's own policy, before the task text reaches a socket.
#[test]
fn a_confidential_building_will_not_name_a_room_with_a_model_off_this_machine() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    lay_rules(dir.path(), "vault", &shut_rules(""));
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    // `.invalid` resolves nowhere on every machine, so the probe fails
    // and the endpoint attaches on the id the person named. What makes
    // it the endpoint this test needs is its host, which is not this
    // machine.
    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("far").unwrap(),
            base_url: "https://digest.invalid/v1".to_owned(),
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: vec!["m-far".to_owned()],
            tuning: channels::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach-far"),
        })
        .unwrap();
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("far").unwrap(),
            model: "m-far".to_owned(),
            tag: kernel::ModelTag::Digest,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select-far"),
        })
        .unwrap();

    let refused = worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("vault").unwrap(),
            task: "the kiln glaze formula nobody outside this house has".to_owned(),
            goal: "it is written down".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap_err();

    assert_eq!(
        *refused.code(),
        AxCode::GateDenied,
        "the book refuses the choice under the building's own policy: {refused}"
    );
    assert!(
        refused.recovery().contains("building/name"),
        "the person is told the two ways out: {}",
        refused.recovery()
    );
    assert!(
        !dir.path().join("vault").join("glaze").exists(),
        "a refused dispatch opened a room anyway"
    );
    assert!(
        !provider.bodies().join("\n").contains("glaze formula"),
        "no call carries the task text of a confidential building"
    );
}

/// The identifier is a function of the job, the address and the
/// millisecond, and nothing else. Two jobs dispatched into the same
/// millisecond are two runs: the hexadecimal round trip this used to
/// make had two failure points that both answered zero, and a city
/// whose runs share an identifier cannot be read back at all.
#[test]
fn two_jobs_at_one_millisecond_get_two_run_ids() {
    let addr = Address::parse("lab/room1").unwrap();
    let now = kernel::TimeMs::new(1_700_000_000_000);
    let job = |hex: &str| kernel::Locator::parse(&format!("file:lab/room1@{}", hex.repeat(20)));
    let one = agreeing::run_id_for(&job("ab").unwrap(), &addr, now);
    let two = agreeing::run_id_for(&job("cd").unwrap(), &addr, now);
    assert_ne!(one, two);
    assert_eq!(
        one,
        agreeing::run_id_for(&job("ab").unwrap(), &addr, now),
        "the same three inputs name the same run, which is what replay rests on"
    );
    assert_ne!(
        one,
        RunId::from_bytes([0u8; 16]),
        "a digest the old code could not print became the all-zero run"
    );
}

/// The instrument `xtask/budgets.toml [prepare_dispatch_ms]` names.
///
/// It pins that the reading is taken and said out loud, not what the
/// reading is: a wall-clock figure belongs to the machine that ran it,
/// so the row records it beside that machine and this test asserts only
/// that a dispatch reports what it spent before the lane took the
/// drive. Run with `--nocapture` to read the figure off this fixture.
#[test]
fn a_dispatch_says_what_it_spent_before_the_drive() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(&["m-local"], vec![completion("done", None)]);

    let written = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&written);
    let mut worker = RunWorker::new(
        dir.path(),
        gateway::Custodian::in_memory(),
        runtime::diagnostics::Diagnostics::new(
            runtime::diagnostics::Level::Trace,
            Box::new(move |entry: runtime::diagnostics::Entry<'_>| {
                sink.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(runtime::diagnostics::render(entry));
            }),
        ),
    )
    .unwrap();
    worker
        .handle(channels::Command::AttachEndpoint {
            name: channels::ProviderName::parse("house").unwrap(),
            base_url,
            dialect: kernel::DialectKind::OpenAi,
            secret: None,
            auth_header: None,
            admit: Vec::new(),
            tuning: channels::EndpointTuning::default(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"attach"),
        })
        .unwrap();
    worker
        .handle(channels::Command::SelectModel {
            endpoint: channels::ProviderName::parse("house").unwrap(),
            model: "m-local".to_owned(),
            tag: kernel::ModelTag::Main,
            context_tokens: kernel::Window::new(32_768),
            max_output_tokens: kernel::Ceiling::new(4_096),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"select"),
        })
        .unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "write one line".to_owned(),
            goal: "the line is written".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let lines = written
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let prepared: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("prepare_dispatch took"))
        .collect();
    let servers: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("mcp_tools took"))
        .collect();
    for line in prepared.iter().chain(servers.iter()) {
        println!("{line}");
    }
    assert_eq!(
        prepared.len(),
        1,
        "one dispatch reports its own preparation once: {lines:#?}"
    );
    assert_eq!(
        servers.len(),
        1,
        "the servers phase reports separately, because a resident connection table would remove only that part: {lines:#?}"
    );
}
