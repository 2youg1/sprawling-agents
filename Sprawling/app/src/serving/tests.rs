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

#[test]
fn the_key_this_city_mints_is_the_key_its_own_door_accepts() {
    let exposed = "0.0.0.0:2501".parse().expect("a literal address");
    let Keyed::Minted(code) = key_for(exposed, None).expect("this machine has entropy") else {
        panic!("an address beyond this machine with nothing configured mints one");
    };
    let digest = wire::PairingToken::from_configured(&code)
        .expect("a minted code is long enough to adopt")
        .digest();
    assert!(
        wire::verify(Some(&code), &digest),
        "the code shown to a person opens the door it guards"
    );
    assert!(!wire::verify(None, &digest), "silence is not the key");
}

#[test]
fn two_serves_of_one_address_mint_two_different_keys() {
    let exposed = "0.0.0.0:2501".parse().expect("a literal address");
    let first = key_for(exposed, None).expect("entropy");
    let second = key_for(exposed, None).expect("entropy");
    assert_ne!(first.code(), second.code());
}

#[test]
fn a_configured_token_is_adopted_rather_than_replaced() {
    let exposed = "0.0.0.0:2501".parse().expect("a literal address");
    let configured = "a-token-the-operator-chose".to_owned();
    assert_eq!(
        key_for(exposed, Some(configured.clone())).expect("no entropy is drawn"),
        Keyed::Adopted(configured)
    );
}

#[test]
fn a_loopback_listener_mints_a_key_nobody_is_shown() {
    let local = "127.0.0.1:2501".parse().expect("a literal address");
    let keyed = key_for(local, None).expect("this machine has entropy");
    assert!(matches!(keyed, Keyed::Unshown(_)));
    assert_eq!(keyed.shown(), None);
    assert!(!keyed.code().is_empty());
}
