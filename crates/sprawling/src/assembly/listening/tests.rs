// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]

/// A serve that cannot take its port leaves the Ledger exactly as it
/// found it. The socket is the first thing a serve takes, so a busy
/// port is refused before a writer exists, rather than after a writer
/// has opened the city and recorded a close nobody asked for.
#[test]
fn a_serve_refused_at_the_socket_writes_no_line() {
    let city = tempfile::tempdir().expect("a temporary directory");
    crate::assembly::init_city(city.path()).expect("a city forms");
    let ledger = kernel::layout::CityLayout::new(city.path()).ledger();
    // As text, so a failure shows the line that was written.
    let lines = || -> Vec<String> {
        storage::read_raw_lines_at(&ledger)
            .expect("the ledger reads")
            .iter()
            .map(|line| String::from_utf8_lossy(line).into_owned())
            .collect()
    };
    let before = lines();
    let taken = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
    let addr = taken.local_addr().expect("the port it took");
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");

    let refused = executor.block_on(super::listen(serving_at(city.path(), addr)));

    assert!(refused.is_err(), "a busy port is a refusal");
    assert_eq!(
        lines(),
        before,
        "a serve refused at the socket wrote into the ledger"
    );
}

/// A serve whose city another writer holds is refused with
/// `E_LEDGER_HELD`, and the refusal hands its port back: the socket is
/// taken first, so a serve that stops at the Ledger must not keep a
/// port nobody will answer on.
#[test]
fn a_serve_of_a_held_city_is_refused_and_lets_its_port_go() {
    let city = tempfile::tempdir().expect("a temporary directory");
    crate::assembly::init_city(city.path()).expect("a city forms");
    let held = storage::JsonlLedger::open(
        &kernel::layout::CityLayout::new(city.path()).ledger(),
        kernel::TimeMs::new(0),
    )
    .expect("the first writer opens the city");
    let addr = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|probe| probe.local_addr())
        .expect("a free port");
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");

    let refused = executor
        .block_on(super::listen(serving_at(city.path(), addr)))
        .err()
        .map(|refusal| refusal.code().as_str());

    assert_eq!(
        refused,
        Some("E_LEDGER_HELD"),
        "a serve opened a city another writer holds"
    );
    assert!(
        std::net::TcpListener::bind(addr).is_ok(),
        "a serve refused at the Ledger kept its port"
    );
    drop(held);
}

/// The phases of opening a served city, in the order `listen` does them
/// (`crates/sprawling/spec/Assembly/Listening.lean` §8-121). The fold
/// names how many lines the city holds, which the test reads off the
/// ledger `init` wrote: one `skill_shelved` per shipped skill moves it.
const OPENING_PHASES: [&str; 7] = [
    "bind",
    "open the ledger",
    "lines from genesis",
    "cut the standing snapshot",
    "cut the views snapshot",
    "copy the views",
    "start the worker",
];

/// A listening city says in one line, at the effect floor and through the
/// same sink as every other diagnostic, what each phase of opening it
/// cost, so the first byte a person waits for can be split by the product
/// itself rather than inferred from outside.
#[test]
fn a_listening_city_says_what_opening_it_cost() {
    let city = tempfile::tempdir().expect("a temporary directory");
    let formed = crate::assembly::init_city(city.path()).expect("a city forms");
    let held = runtime::replay::verify_ledger_dir(&formed.ledger_dir)
        .expect("the formed ledger verifies")
        .raw_lines()
        .len();
    let journal = crate::serving::Journal::new(std::sync::Arc::new(crate::assembly::SystemClock));
    let mut heard = journal.lines().subscribe();
    let addr = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|probe| probe.local_addr())
        .expect("a free port");
    let serving = super::Serving {
        log: runtime::diagnostics::Diagnostics::new(
            runtime::diagnostics::Level::Effect,
            journal.sink(),
        ),
        journal,
        ..serving_at(city.path(), addr)
    };
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");

    let listening = executor
        .block_on(super::listen(serving))
        .expect("the city listens");

    let mut opening = Vec::new();
    while let Ok(line) = heard.try_recv() {
        if line.level == wire::LogLevel::Effect && line.line.starts_with("opened the city in ") {
            opening.push(line.line);
        }
    }
    // Where each phase's name falls in the line, so the durations, which
    // differ on every run, stay out of the comparison.
    let named_in_order = |line: &str| -> Vec<&'static str> {
        let mut found: Vec<(usize, &'static str)> = OPENING_PHASES
            .iter()
            .filter_map(|phase| line.find(phase).map(|at| (at, *phase)))
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, phase)| phase).collect()
    };
    assert_eq!(
        (
            opening.len(),
            opening
                .first()
                .map(|line| named_in_order(line.as_str()))
                .unwrap_or_default()
        ),
        (1, OPENING_PHASES.to_vec()),
        "the opening lines heard: {opening:?}"
    );
    assert!(
        opening
            .iter()
            .all(|line| line.contains(&format!("fold {held} lines from genesis"))),
        "the opening lines heard: {opening:?}"
    );
    drop(listening);
}

fn serving_at(city_root: &std::path::Path, addr: std::net::SocketAddr) -> super::Serving {
    super::Serving {
        city_root: city_root.to_path_buf(),
        addr,
        token: "a-key-for-this-test-serve".to_owned(),
        client: wire::ClientAssets::Disk(city_root.to_path_buf()),
        vault: gateway::Custodian::in_memory(),
        vault_notice: None,
        log: runtime::diagnostics::Diagnostics::off(),
        journal: crate::serving::Journal::new(std::sync::Arc::new(crate::assembly::SystemClock)),
        core: accounting::person::CorePriority::Normal,
    }
}
