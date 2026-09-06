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
