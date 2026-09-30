// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The six properties `adversary/design/RemoteDoor.lean` proves, held
//! against the Rust door one scenario each.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

use kernel::{AxCode, TimeMs};

use super::*;
use crate::pairing::PairingCode;

fn long() -> TimeMs {
    TimeMs::new(1_000_000)
}

fn open_door() -> Door {
    let mut door = Door::start(Vec::new(), Epoch::from_entropy([0; 16]));
    door.open(Epoch::from_entropy([1; 16]), long()).unwrap();
    door
}

fn key() -> DeviceKey {
    DeviceKey::from_bytes(vec![9; 32])
}

/// A door with one paired device, `phone`, which acts.
fn paired(authority: Authority) -> (Door, DeviceId) {
    let mut door = open_door();
    let (code, _) = PairingCode::mint([3; 16]);
    door.expect_pairing(
        code,
        DeviceName::parse("phone").unwrap(),
        authority,
        TimeMs::new(600),
    )
    .unwrap();
    let id = DeviceId::from_entropy([4; 16]);
    door.pair(code, id, key(), TimeMs::new(10)).unwrap();
    (door, id)
}

#[test]
fn a_closed_door_admits_nothing() {
    let (mut door, id) = paired(Authority::Act);
    door.close();
    let (code, _) = PairingCode::mint([5; 16]);
    let refused = [
        door.expect_pairing(
            code,
            DeviceName::parse("tablet").unwrap(),
            Authority::Act,
            long(),
        ),
        door.pair(
            code,
            DeviceId::from_entropy([6; 16]),
            key(),
            TimeMs::new(20),
        ),
        door.admit(SessionId::from_entropy([7; 16]), id, long()),
    ];
    assert!(
        refused.iter().all(|each| each
            .as_ref()
            .is_err_and(|err| *err.code() == AxCode::GateDenied)),
        "{refused:?}"
    );
}

#[test]
fn closing_ends_every_session_and_opening_again_brings_none_back() {
    let (mut door, id) = paired(Authority::Act);
    let session = SessionId::from_entropy([7; 16]);
    door.admit(session, id, long()).unwrap();
    assert_eq!(
        door.authority(session, TimeMs::new(20)),
        Some(Authority::Act)
    );
    door.close();
    let closed = door.authority(session, TimeMs::new(20));
    door.open(Epoch::from_entropy([2; 16]), long()).unwrap();
    assert_eq!(
        [closed, door.authority(session, TimeMs::new(20))],
        [None, None]
    );
}

#[test]
fn a_code_pairs_once_in_its_own_epoch_before_it_expires() {
    let (mut door, _) = paired(Authority::Act);
    let (used, _) = PairingCode::mint([3; 16]);
    let again = door.pair(
        used,
        DeviceId::from_entropy([8; 16]),
        key(),
        TimeMs::new(20),
    );

    let (stale, _) = PairingCode::mint([10; 16]);
    door.expect_pairing(
        stale,
        DeviceName::parse("old").unwrap(),
        Authority::Act,
        long(),
    )
    .unwrap();
    door.close();
    door.open(Epoch::from_entropy([2; 16]), long()).unwrap();
    let old_epoch = door.pair(
        stale,
        DeviceId::from_entropy([11; 16]),
        key(),
        TimeMs::new(20),
    );

    let (late, _) = PairingCode::mint([12; 16]);
    door.expect_pairing(
        late,
        DeviceName::parse("late").unwrap(),
        Authority::Act,
        TimeMs::new(30),
    )
    .unwrap();
    let expired = door.pair(
        late,
        DeviceId::from_entropy([13; 16]),
        key(),
        TimeMs::new(30),
    );

    assert!([again, old_epoch, expired].iter().all(Result::is_err));
    assert_eq!(door.devices().len(), 1);
}

#[test]
fn a_revoked_device_holds_no_session_and_opens_none() {
    let (mut door, id) = paired(Authority::Act);
    let session = SessionId::from_entropy([7; 16]);
    door.admit(session, id, long()).unwrap();
    door.revoke(id).unwrap();
    assert_eq!(door.authority(session, TimeMs::new(20)), None);
    assert!(
        door.admit(SessionId::from_entropy([8; 16]), id, long())
            .is_err()
    );
}

#[test]
fn a_session_never_outlives_the_door() {
    let mut door = Door::start(Vec::new(), Epoch::from_entropy([0; 16]));
    door.open(Epoch::from_entropy([1; 16]), TimeMs::new(100))
        .unwrap();
    let (code, _) = PairingCode::mint([3; 16]);
    door.expect_pairing(
        code,
        DeviceName::parse("phone").unwrap(),
        Authority::Watch,
        TimeMs::new(50),
    )
    .unwrap();
    let id = DeviceId::from_entropy([4; 16]);
    door.pair(code, id, key(), TimeMs::new(10)).unwrap();
    let session = SessionId::from_entropy([7; 16]);
    door.admit(session, id, long()).unwrap();
    let before = door.authority(session, TimeMs::new(99));
    let at = door.authority(session, TimeMs::new(100));
    door.tick(TimeMs::new(100));
    assert_eq!(
        [before, at, door.closes_at().map(|_| Authority::Watch)],
        [Some(Authority::Watch), None, None]
    );
}

#[test]
fn a_remote_session_never_reaches_a_local_only_verb() {
    let every = [Authority::Watch, Authority::Act].map(|each| {
        [VerbClass::Read, VerbClass::Act, VerbClass::LocalOnly].map(|class| permits(each, class))
    });
    assert_eq!(every, [[true, false, false], [true, true, false]]);
}

#[test]
fn the_door_refuses_to_reopen_in_the_epoch_it_stands_in() {
    let mut door = open_door();
    assert!(door.open(Epoch::from_entropy([1; 16]), long()).is_err());
}
