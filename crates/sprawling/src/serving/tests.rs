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

use super::door::*;

/// **The property the whole card exists for.** A minted code has to
/// be the code that opens the door, and the two halves reach that
/// digest by different routes: `PairingToken::mint` hashes the text
/// it just produced, while `serve` hashes the text it is handed back
/// through `from_configured`. If those ever stop agreeing, an
/// exposed city refuses the person who started it, holding a key it
/// minted for them.
#[test]
fn the_key_this_city_mints_is_the_key_its_own_door_accepts() {
    let exposed = "0.0.0.0:8787".parse().expect("a literal address");
    let Keyed::Minted(code) = key_for(exposed, None).expect("this machine has entropy") else {
        panic!("an address beyond this machine with nothing configured mints one");
    };
    let digest = channels::PairingToken::from_configured(&code)
        .expect("a minted code is long enough to adopt")
        .digest();
    assert!(
        channels::verify(Some(&code), &digest),
        "the code shown to a person opens the door it guards"
    );
    assert!(!channels::verify(None, &digest), "silence is not the key");
}

/// One-time means one time. Two serves of the same address must not
/// produce the same code, or a key read off yesterday's terminal
/// still works today.
#[test]
fn two_serves_of_one_address_mint_two_different_keys() {
    let exposed = "0.0.0.0:8787".parse().expect("a literal address");
    let first = key_for(exposed, None).expect("entropy");
    let second = key_for(exposed, None).expect("entropy");
    assert_ne!(first.code(), second.code());
}

/// What the operator configured is adopted, never replaced. Minting
/// over it would break every browser already paired with this city.
#[test]
fn a_configured_token_is_adopted_rather_than_replaced() {
    let exposed = "0.0.0.0:8787".parse().expect("a literal address");
    let configured = "a-token-the-operator-chose".to_owned();
    assert_eq!(
        key_for(exposed, Some(configured.clone())).expect("no entropy is drawn"),
        Keyed::Adopted(configured)
    );
}

/// Loopback stays frictionless: nothing is minted and nothing is
/// asked for, which is the property `decide_bind` is written to keep.
#[test]
fn a_loopback_listener_is_handed_nothing_to_present() {
    let local = "127.0.0.1:8787".parse().expect("a literal address");
    let keyed = key_for(local, None).expect("no entropy is drawn");
    assert_eq!(keyed, Keyed::NothingToPresent);
    assert_eq!(keyed.code(), None);
}

/// A serve that cannot take its port leaves the Ledger exactly as it
/// found it. The socket is the first thing a serve takes, so a busy
/// port is refused before a writer exists, rather than after a writer
/// has opened the city and recorded a close nobody asked for.
#[test]
fn a_serve_refused_at_the_socket_writes_no_line() {
    let city = tempfile::tempdir().expect("a temporary directory");
    crate::assembly::init_city(city.path()).expect("a city forms");
    let ledger = crate::assembly::ledger_dir(city.path());
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

    let refused = executor.block_on(super::listen(super::Serving {
        city_root: city.path().to_path_buf(),
        addr,
        token: None,
        client: channels::ClientAssets::Disk(city.path().to_path_buf()),
        vault: gateway::Custodian::in_memory(),
        vault_notice: None,
        log: runtime::diagnostics::Diagnostics::off(),
        journal: super::Journal::new(),
        console: None,
    }));

    assert!(refused.is_err(), "a busy port is a refusal");
    assert_eq!(
        lines(),
        before,
        "a serve refused at the socket wrote into the ledger"
    );
}
