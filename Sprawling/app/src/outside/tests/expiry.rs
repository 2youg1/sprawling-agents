// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::net::UdpSocket;
use std::time::Duration;

use kernel::event::record::{RemoteClosed, RemoteClosing};
use kernel::{EventRecord, Ledger};

use super::super::listener::{Reaching, open};
use super::*;

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

const NAP: Duration = Duration::from_millis(50);
const NAPS: u32 = 400;

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
    let now = Arc::new(AtomicU64::new(1_000));
    let turned = Arc::clone(&now);
    let doorway = Doorway::keep(Keeping {
        devices: dir.path().join("remote").join("devices.toml"),
        ledger: Box::new(records.clone()),
        senses: Senses {
            clock: Arc::new(move || Ok(TimeMs::new(turned.load(Ordering::SeqCst)))),
            ..senses(1)
        },
        key: CityKey::of(vault(), Some(B3Hash::digest(b"genesis"))).unwrap(),
    })
    .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap();
    drop(socket);
    let file = kernel::layout::CityLayout::new(dir.path()).city_config();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let identity = wtransport::Identity::self_signed(["127.0.0.1"]).unwrap();
    let cert = dir.path().join("certificate.pem");
    let key = dir.path().join("private.pem");
    runtime.block_on(async {
        identity
            .certificate_chain()
            .store_pemfile(&cert)
            .await
            .unwrap();
        identity
            .private_key()
            .store_secret_pemfile(&key)
            .await
            .unwrap();
    });
    std::fs::write(file, format!(
        "[remote]\nroute = 'h3'\nurl = 'https://{port}'\nlisten = '{port}'\ncertificate = '{}'\nprivate_key = '{}'\n",
        cert.display(), key.display()
    )).unwrap();
    let reaching = Reaching {
        runtime: runtime.handle().clone(),
        city_root: dir.path().to_path_buf(),
        city: port,
        token: "city-key".to_owned(),
        page: Arc::new(wire::ClientAssets::Embedded(&[])),
    };
    let lasting = Lasting::of_ms(60_000).unwrap();
    open(&doorway, &reaching, lasting).unwrap();
    let bound_while_open = UdpSocket::bind(port).is_err();

    now.store(1_000 + 60_000, Ordering::SeqCst);
    let closed = within(|| records.0.lock().unwrap().len() >= 2);
    let refused_after = within(|| UdpSocket::bind(port).is_ok());

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
            bound_while_open,
            closed,
            written,
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
            true,
            true,
        )
    );
}
