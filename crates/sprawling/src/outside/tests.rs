// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote door as a served city keeps it, driven by a device written
//! in Rust against a scripted route, on a counted clock and a counted
//! random source (`crates/sprawling/Spec.lean` §8-139, §8-140).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use kernel::{AxCode, AxError, B3Hash, EventDraft, EventKind, EventRef, Ledger, Seq, TimeMs};
use remote_access::door::{Authority, DeviceId};
use remote_access::handshake::{self, Session};
use remote_access::keys::SigningKey;
use remote_access::route::scripted::ScriptedRoute;
use remote_access::route::{Opened, Permanence, PublicUrl};
use remote_access::seal::Payload;

use super::conduit::{Conduit, Step};
use super::console::{Lasting, RemoteLine, parse};
use super::keeper::{CityKey, Doorway, Keeping, Revoking, Senses};

mod expiry;

/// Every kind a door wrote, in the order it wrote them.
#[derive(Clone, Default)]
struct Written(Arc<Mutex<Vec<EventKind>>>);

impl Ledger for Written {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let record = kernel::EventRecord::from_draft(draft, Seq::FIRST, B3Hash::digest(b"prev"));
        self.0.lock().unwrap().push(record.kind());
        Ok(record.to_ref())
    }
}

impl Written {
    fn kinds(&self) -> Vec<EventKind> {
        self.0.lock().unwrap().clone()
    }
}

/// A clock that moves one second per reading, and a random source that
/// never repeats itself from its `first` draw on.
fn senses(first: u64) -> Senses {
    let ticks = Arc::new(AtomicU64::new(1_000));
    let draws = Arc::new(AtomicU64::new(first));
    Senses {
        clock: Arc::new(move || Ok(TimeMs::new(ticks.fetch_add(1_000, Ordering::SeqCst)))),
        entropy: Arc::new(move |bytes: &mut [u8]| {
            let draw = draws.fetch_add(1, Ordering::SeqCst).to_be_bytes();
            for (at, byte) in bytes.iter_mut().enumerate() {
                *byte = draw[at % draw.len()] ^ u8::try_from(at % 251).unwrap();
            }
            Ok(())
        }),
    }
}

fn opened() -> Opened {
    Opened {
        url: PublicUrl::parse("https://city.example").unwrap(),
        permanence: Permanence::Fixed,
    }
}

/// The vault one city keeps its key in, across every start of it.
type Vault = Arc<Mutex<gateway::Custodian>>;

fn vault() -> Vault {
    Arc::new(Mutex::new(gateway::Custodian::in_memory()))
}

fn kept(devices: &std::path::Path, written: &Written) -> Doorway {
    started(devices, written, &vault(), 1)
}

/// One start of a city whose vault is `vault`, its random source drawing
/// from `first` on.
fn started(devices: &std::path::Path, written: &Written, vault: &Vault, first: u64) -> Doorway {
    Doorway::keep(Keeping {
        devices: devices.join("remote").join("devices.toml"),
        ledger: Box::new(written.clone()),
        choose: Box::new(|| Ok(Box::new(ScriptedRoute::new(opened())))),
        senses: senses(first),
        key: CityKey::of(Arc::clone(vault), Some(B3Hash::digest(b"genesis"))).unwrap(),
    })
    .unwrap()
}

fn lasting(length: &str) -> Lasting {
    Lasting::read(length).unwrap()
}

fn local() -> SocketAddr {
    "127.0.0.1:9".parse().unwrap()
}

/// A device on the other side: its key and the id the city gave it.
struct Device {
    key: SigningKey,
    id: DeviceId,
    city: remote_access::keys::VerifyingKey,
}

/// Pairs a device called `name` through the door, as a page would.
fn pair(doorway: &Doorway, name: &str, authority: Authority, seed: u8) -> Device {
    let inviting = doorway.invite(name, authority).unwrap();
    let key = SigningKey::from_seed(&[seed; 32]).unwrap();
    let pairing = handshake::device_pair_hello([seed; 32]).unwrap();
    let (reply, city_side) = doorway.pair_reply(pairing.hello()).unwrap();
    let claimed = pairing.claim(&reply, &inviting.invitation, &key).unwrap();
    let mut sealed_session = claimed.session;
    let answer = doorway.claim(city_side, &claimed.sealed).unwrap();
    let id = sealed_session.opener.open(&answer).unwrap();
    Device {
        key,
        id: DeviceId::from_entropy(<[u8; 16]>::try_from(id).unwrap()),
        city: claimed.city,
    }
}

/// Opens a session for a paired device, and the device's half of it.
fn admit(doorway: &Doorway, device: &Device, token: Option<&str>) -> (Conduit, Session) {
    let waiting = handshake::device_hello(device.id, [7; 32]).unwrap();
    let (reply, city_side) = doorway.session_reply(waiting.hello()).unwrap();
    let (finish, mine) = waiting.finish(&reply, &device.city, &device.key).unwrap();
    let admitted = doorway.admit(city_side, &finish).unwrap();
    (
        Conduit::new(doorway.clone(), admitted, token.map(str::to_owned)),
        mine,
    )
}

/// The code of the refusal the device was answered with, or `None` when
/// the frame was not answered with one.
fn refusal(mine: &mut Session, step: Step) -> Option<AxCode> {
    let Step::Answer(sealed) = step else {
        return None;
    };
    let Payload::Frame(text) = Payload::from_bytes(&mine.opener.open(&sealed).unwrap()).unwrap()
    else {
        return None;
    };
    match serde_json::from_str(&text).unwrap() {
        wire::ServerFrame::Refusal(refused) => Some(*refused.code()),
        _ => None,
    }
}

fn sealed(mine: &mut Session, frame: &wire::ClientFrame) -> (String, Vec<u8>) {
    let text = serde_json::to_string(frame).unwrap();
    let bytes = mine
        .sealer
        .seal(&Payload::Frame(text.clone()).to_bytes())
        .unwrap();
    (text, bytes)
}

fn command(command: wire::WireCommand) -> wire::ClientFrame {
    wire::ClientFrame::Command(Box::new(command))
}

fn idem() -> kernel::IdemKey {
    kernel::IdemKey::derive(&kernel::RunId::CITY, Seq::FIRST, &[3; 16])
}

#[test]
fn the_door_writes_its_five_lines_in_the_order_they_happen() {
    let dir = tempfile::tempdir().unwrap();
    let written = Written::default();
    let doorway = kept(dir.path(), &written);
    doorway.open(local(), lasting("2h")).unwrap();
    let phone = pair(&doorway, "phone", Authority::Act, 1);
    drop(admit(&doorway, &phone, None));
    doorway
        .revoke(&Revoking::Named("phone".to_owned()))
        .unwrap();
    doorway
        .close(kernel::event::record::RemoteClosing::Console)
        .unwrap();
    assert_eq!(
        written.kinds(),
        [
            EventKind::RemoteOpened,
            EventKind::DevicePaired,
            EventKind::RemoteSessionStarted,
            EventKind::DeviceRevoked,
            EventKind::RemoteClosed,
        ]
    );
}

#[test]
fn a_local_only_frame_is_refused_and_reaches_no_city() {
    let dir = tempfile::tempdir().unwrap();
    let doorway = kept(dir.path(), &Written::default());
    doorway.open(local(), lasting("1h")).unwrap();
    let laptop = pair(&doorway, "laptop", Authority::Act, 2);
    let (mut conduit, mut mine) = admit(&doorway, &laptop, None);
    let reveal = command(wire::WireCommand::Reveal {
        at: kernel::Address::parse("lab").unwrap(),
        idem: idem(),
    });
    let (_, bytes) = sealed(&mut mine, &reveal);
    let step = conduit.judge(&bytes).unwrap();
    assert_eq!(refusal(&mut mine, step), Some(AxCode::GateDenied));
}

/// The privacy page reads and changes the host, not the city: a device
/// that may act is refused both the question and the operation, and
/// neither reaches the city (`crates/wire/Spec.lean` §8-86, §8-87).
#[test]
fn the_privacy_page_is_refused_to_a_device_that_may_act() {
    let dir = tempfile::tempdir().unwrap();
    let doorway = kept(dir.path(), &Written::default());
    doorway.open(local(), lasting("1h")).unwrap();
    let laptop = pair(&doorway, "laptop", Authority::Act, 2);
    let (mut conduit, mut mine) = admit(&doorway, &laptop, None);
    let ask = wire::ClientFrame::Ask(wire::Ask {
        ask_id: wire::AskId(1),
        query: wire::Query::Privacy,
    });
    let operation = command(wire::WireCommand::PrivacyOperation(wire::PrivacyRequest {
        action: wire::PrivacyAction::Apply {
            control: wire::PrivacyControl::StartLaunchTracking,
            expected: wire::PrivacyValue::Absent,
        },
        idem: idem(),
    }));
    for frame in [ask, operation] {
        let (_, bytes) = sealed(&mut mine, &frame);
        let step = conduit.judge(&bytes).unwrap();
        assert_eq!(
            refusal(&mut mine, step),
            Some(AxCode::GateDenied),
            "{frame:?}"
        );
    }
}

#[test]
fn a_watching_device_is_refused_a_verb_that_acts_and_may_still_ask() {
    let dir = tempfile::tempdir().unwrap();
    let doorway = kept(dir.path(), &Written::default());
    doorway.open(local(), lasting("1h")).unwrap();
    let tablet = pair(&doorway, "tablet", Authority::Watch, 3);
    let (mut conduit, mut mine) = admit(&doorway, &tablet, Some("city-token"));
    let cancel = command(wire::WireCommand::Cancel {
        run: kernel::RunId::CITY,
        idem: idem(),
    });
    let ask = wire::ClientFrame::Ask(wire::Ask {
        ask_id: wire::AskId(1),
        query: wire::Query::Metrics,
    });
    let (_, cancelling) = sealed(&mut mine, &cancel);
    let step = conduit.judge(&cancelling).unwrap();
    let refused = refusal(&mut mine, step);
    let (asked, asking) = sealed(&mut mine, &ask);
    assert_eq!(
        (refused, conduit.judge(&asking).unwrap()),
        (Some(AxCode::GateDenied), Step::Forward(asked))
    );
}

#[test]
fn the_device_table_survives_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let doorway = kept(dir.path(), &Written::default());
    doorway.open(local(), lasting("1h")).unwrap();
    pair(&doorway, "phone", Authority::Watch, 4);
    pair(&doorway, "客厅的 iPad", Authority::Act, 5);
    let before = doorway.devices().unwrap();
    drop(doorway);
    let after = kept(dir.path(), &Written::default()).devices().unwrap();
    assert_eq!((before.len(), after), (2, before));
}

#[test]
fn a_restarted_city_keeps_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let vault = vault();
    let first = started(dir.path(), &Written::default(), &vault, 1);
    first.open(local(), lasting("1h")).unwrap();
    let phone = pair(&first, "phone", Authority::Act, 6);
    drop(first);
    let again = started(dir.path(), &Written::default(), &vault, 1_000);
    again.open(local(), lasting("1h")).unwrap();
    let tablet = pair(&again, "tablet", Authority::Act, 7);
    assert_eq!(tablet.city, phone.city);
}

#[test]
fn replacing_the_key_unpairs_every_device() {
    let dir = tempfile::tempdir().unwrap();
    let written = Written::default();
    let vault = vault();
    let doorway = started(dir.path(), &written, &vault, 1);
    doorway.open(local(), lasting("1h")).unwrap();
    let phone = pair(&doorway, "phone", Authority::Act, 8);
    pair(&doorway, "tablet", Authority::Watch, 9);
    doorway
        .revoke(&Revoking::Named("tablet".to_owned()))
        .unwrap();
    let while_open = doorway.replace_key().err().map(|refused| *refused.code());
    doorway
        .close(kernel::event::record::RemoteClosing::Console)
        .unwrap();
    let unpaired: Vec<String> = doorway
        .replace_key()
        .unwrap()
        .iter()
        .map(|device| device.name.as_str().to_owned())
        .collect();
    drop(doorway);
    let restarted = started(dir.path(), &Written::default(), &vault, 2_000);
    let before_pairing = restarted.devices().unwrap().len();
    restarted.open(local(), lasting("1h")).unwrap();
    let same_city = restarted
        .invite("phone", Authority::Act)
        .ok()
        .map(|inviting| inviting.invitation.city == handshake::CityFingerprint::of(&phone.city));
    let revoked_lines = written
        .kinds()
        .iter()
        .filter(|kind| **kind == EventKind::DeviceRevoked)
        .count();
    assert_eq!(
        (
            while_open,
            unpaired,
            before_pairing,
            same_city,
            revoked_lines
        ),
        (
            Some(AxCode::Busy),
            vec!["phone".to_owned()],
            0,
            Some(false),
            2
        )
    );
}

#[test]
fn a_remote_line_reads_as_the_verb_it_names() {
    assert_eq!(
        (
            match parse("open --for 30m") {
                RemoteLine::Open(lasting) => lasting.ms(),
                _ => 0,
            },
            [
                parse("pair 客厅的 iPad --watch"),
                parse("revoke --all"),
                parse("replace-key"),
                parse("close now"),
                parse("open --for 8w"),
            ]
        ),
        (
            1_800_000,
            [
                RemoteLine::Pair {
                    name: "客厅的 iPad".to_owned(),
                    authority: Authority::Watch,
                },
                RemoteLine::Revoke(Revoking::All),
                RemoteLine::ReplaceKey,
                RemoteLine::Unreadable,
                RemoteLine::Unreadable,
            ]
        )
    );
}
