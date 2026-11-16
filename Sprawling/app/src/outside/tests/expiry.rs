// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The door closes itself when its time is up, through the listener's
//! own tick (`crates/sprawling/spec/Outside/Conduit.lean` §8-139).
//!
//! Everything from `listener::open` on is the production path; the
//! clock is one the test turns, and the route is a stand-in that
//! remembers the port it was asked to reach and how often it closed.

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use kernel::event::record::{RemoteClosed, RemoteClosing};
use kernel::{EventRecord, Ledger};
use remote_access::route::Route;

use super::super::listener::{Reaching, open};
use super::*;

/// Every record a door wrote, whole, so a reader can see why it closed.
#[derive(Clone, Default)]
struct Records(Arc<Mutex<Vec<EventRecord>>>);

impl Ledger for Records {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let record = EventRecord::from_draft(draft, Seq::FIRST, B3Hash::digest(b"prev"));
        let at = record.to_ref();
        self.0.lock().unwrap().push(record);
        Ok(at)
    }
}

/// What the stand-in route saw: the loopback port it was opened to, and
/// how many times it was closed.
#[derive(Clone, Default)]
struct Seen(Arc<Mutex<(Option<SocketAddr>, u32)>>);

struct StandIn(Seen);

impl Route for StandIn {
    fn open(&mut self, local: SocketAddr) -> Result<Opened, AxError> {
        self.0.0.lock().unwrap().0 = Some(local);
        Ok(opened())
    }

    fn close(&mut self) -> Result<(), AxError> {
        self.0.0.lock().unwrap().1 += 1;
        Ok(())
    }
}

/// How often, and how many times, a check looks again: twenty seconds
/// for the listener's one-second tick to notice, counted in naps rather
/// than read off a clock this crate keeps to `bin::assembly`.
const NAP: Duration = Duration::from_millis(50);
const NAPS: u32 = 400;

/// Looks until `done` holds or the naps run out, and says which.
fn within(done: impl Fn() -> bool) -> bool {
    (0..NAPS).any(|_| {
        done() || {
            std::thread::sleep(NAP);
            false
        }
    }) || done()
}

#[test]
fn an_open_door_closes_itself_as_expired_when_the_clock_passes_its_time() {
    let dir = tempfile::tempdir().unwrap();
    let records = Records::default();
    let seen = Seen::default();
    let now = Arc::new(AtomicU64::new(1_000));
    let turned = Arc::clone(&now);
    let chosen = seen.clone();
    let doorway = Doorway::keep(Keeping {
        devices: dir.path().join("remote").join("devices.toml"),
        ledger: Box::new(records.clone()),
        choose: Box::new(move || Ok(Box::new(StandIn(chosen.clone())))),
        senses: Senses {
            clock: Arc::new(move || Ok(TimeMs::new(turned.load(Ordering::SeqCst)))),
            ..senses(1)
        },
        key: CityKey::of(vault(), Some(B3Hash::digest(b"genesis"))).unwrap(),
    })
    .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let reaching = Reaching {
        runtime: runtime.handle().clone(),
        city: local(),
        token: "city-key".to_owned(),
        page: Arc::new(wire::ClientAssets::Embedded(&[])),
    };
    let lasting = Lasting::of_ms(60_000).unwrap();
    open(&doorway, &reaching, lasting).unwrap();
    let port = seen.0.lock().unwrap().0.unwrap();
    let answered_while_open = TcpStream::connect(port).is_ok();

    now.store(1_000 + 60_000, Ordering::SeqCst);
    let closed = within(|| records.0.lock().unwrap().len() >= 2);
    let refused_after = within(|| TcpStream::connect(port).is_err());

    let written: Vec<(EventKind, Option<RemoteClosing>)> = records
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|record| {
            let why = record.data().read::<RemoteClosed>().ok().map(|c| c.why);
            (record.kind(), why)
        })
        .collect();
    let invited = doorway
        .invite("phone", Authority::Act)
        .err()
        .map(|refused| *refused.code());
    assert_eq!(
        (
            answered_while_open,
            closed,
            written,
            seen.0.lock().unwrap().1,
            refused_after,
            invited.is_some(),
        ),
        (
            true,
            true,
            vec![
                (EventKind::RemoteOpened, None),
                (EventKind::RemoteClosed, Some(RemoteClosing::Expired)),
            ],
            1,
            true,
            true,
        )
    );
}
