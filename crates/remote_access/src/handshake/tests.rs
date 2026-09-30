// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A paired device and its city agree two session keys, and anyone in
//! between who is not one of them fails at the step that proves it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;
use crate::keys::SEED_BYTES;

fn city() -> SigningKey {
    SigningKey::from_seed(&[1; SEED_BYTES]).unwrap()
}

fn device() -> SigningKey {
    SigningKey::from_seed(&[2; SEED_BYTES]).unwrap()
}

fn id() -> DeviceId {
    DeviceId::from_entropy([3; 16])
}

#[test]
fn both_sides_seal_what_the_other_opens() {
    let waiting = device_hello(id(), [4; NONCE_BYTES]).unwrap();
    let (reply, city_waiting) = city_reply(waiting.hello(), &city(), [5; NONCE_BYTES]).unwrap();
    let (finish, mut on_device) = waiting.finish(&reply, &city().public(), &device()).unwrap();
    let mut on_city = city_waiting.accept(&finish, &device().public()).unwrap();

    let up = on_device.sealer.seal(b"dispatch").unwrap();
    let down = on_city.sealer.seal(b"answered").unwrap();
    assert_eq!(
        [
            on_city.opener.open(&up).unwrap(),
            on_device.opener.open(&down).unwrap()
        ],
        [b"dispatch".to_vec(), b"answered".to_vec()]
    );
}

#[test]
fn a_device_refuses_a_city_it_did_not_pin() {
    let waiting = device_hello(id(), [4; NONCE_BYTES]).unwrap();
    let impostor = SigningKey::from_seed(&[9; SEED_BYTES]).unwrap();
    let (reply, _) = city_reply(waiting.hello(), &impostor, [5; NONCE_BYTES]).unwrap();
    assert!(waiting.finish(&reply, &city().public(), &device()).is_err());
}

#[test]
fn a_city_refuses_a_device_that_holds_another_key() {
    let waiting = device_hello(id(), [4; NONCE_BYTES]).unwrap();
    let (reply, city_waiting) = city_reply(waiting.hello(), &city(), [5; NONCE_BYTES]).unwrap();
    let thief = SigningKey::from_seed(&[8; SEED_BYTES]).unwrap();
    let (finish, _) = waiting.finish(&reply, &city().public(), &thief).unwrap();
    assert!(city_waiting.accept(&finish, &device().public()).is_err());
}

#[test]
fn a_reply_changed_on_the_way_is_refused() {
    let waiting = device_hello(id(), [4; NONCE_BYTES]).unwrap();
    let (reply, _) = city_reply(waiting.hello(), &city(), [5; NONCE_BYTES]).unwrap();
    let mut bytes = reply.as_bytes().to_vec();
    bytes[40] ^= 1;
    let changed = Reply::from_bytes(&bytes).unwrap();
    assert!(
        waiting
            .finish(&changed, &city().public(), &device())
            .is_err()
    );
}

#[test]
fn a_finish_from_one_handshake_does_not_complete_another() {
    let first = device_hello(id(), [4; NONCE_BYTES]).unwrap();
    let (first_reply, _) = city_reply(first.hello(), &city(), [5; NONCE_BYTES]).unwrap();
    let (old_finish, _) = first
        .finish(&first_reply, &city().public(), &device())
        .unwrap();

    let second = device_hello(id(), [4; NONCE_BYTES]).unwrap();
    let (_, city_waiting) = city_reply(second.hello(), &city(), [5; NONCE_BYTES]).unwrap();
    assert!(
        city_waiting
            .accept(&old_finish, &device().public())
            .is_err()
    );
}

#[test]
fn the_hello_names_its_device() {
    let waiting = device_hello(id(), [4; NONCE_BYTES]).unwrap();
    assert_eq!(
        Hello::from_bytes(waiting.hello().as_bytes())
            .unwrap()
            .device(),
        id()
    );
}
