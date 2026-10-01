// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote door as a served city keeps it, driven by a device written
//! in Rust against a scripted route, on a counted clock and a counted
//! random source (sprawling-SPEC.md 8-139, 8-140).

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
use super::keeper::{Doorway, Keeping, Revoking, Senses};

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
/// never repeats itself.
fn senses() -> Senses {
    let ticks = Arc::new(AtomicU64::new(1_000));
    let draws = Arc::new(AtomicU64::new(1));
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

fn kept(devices: &std::path::Path, written: &Written) -> Doorway {
    Doorway::keep(Keeping {
        devices: devices.join("remote").join("devices.toml"),
        ledger: Box::new(written.clone()),
        route: Some(Box::new(ScriptedRoute::new(opened()))),
        senses: senses(),
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
                RemoteLine::Unreadable,
                RemoteLine::Unreadable,
            ]
        )
    );
}
