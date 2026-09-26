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
        memory::read_raw_lines_at(&ledger)
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
    let held = memory::JsonlLedger::open(
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

fn serving_at(city_root: &std::path::Path, addr: std::net::SocketAddr) -> super::Serving {
    super::Serving {
        city_root: city_root.to_path_buf(),
        addr,
        token: None,
        client: channels::ClientAssets::Disk(city_root.to_path_buf()),
        vault: gateway::Custodian::in_memory(),
        vault_notice: None,
        log: runtime::diagnostics::Diagnostics::off(),
        journal: crate::serving::Journal::new(std::sync::Arc::new(crate::assembly::SystemClock)),
        console: None,
    }
}
