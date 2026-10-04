// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Background conformance for city D19 and `crates/sprawling/spec/Assembly.lean`.

use super::*;
use kernel::event::record::AuditVerdict;
use kernel::{Address, Ledger, TimeMs};

fn draft(kind: EventKind, data: Payload) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(0),
        who: "city".into(),
        addr: None,
        kind,
        data,
        ig: false,
    }
}

fn open(root: &Path) -> storage::JsonlLedger {
    storage::JsonlLedger::open(
        &kernel::layout::CityLayout::new(root).ledger(),
        TimeMs::new(0),
    )
    .unwrap()
    .0
}

fn shelving(ledger: &mut impl Ledger, digest: B3Hash, source: ShelvedFrom) {
    ledger
        .append(draft(
            EventKind::SkillShelved,
            Payload::of(&SkillShelved {
                skill: "kiln".into(),
                digest,
                source,
            })
            .unwrap(),
        ))
        .unwrap();
}

fn landed(root: &Path, slot: &city::Slot, body: &str) -> (PathBuf, B3Hash) {
    let source = tempfile::tempdir().unwrap();
    let path = source.path().join("kiln.md");
    std::fs::write(&path, body).unwrap();
    let installed =
        city::install_skill(root, slot, &path, &mut |bytes| Ok(B3Hash::digest(bytes))).unwrap();
    (
        root.join(installed.holding.shelf.address().unwrap().as_str()),
        installed.hash,
    )
}

fn remote(name: &str) -> ShelvedFrom {
    ShelvedFrom::SkillsSh { name: name.into() }
}

#[test]
fn background_sources_belong_to_each_content_digest() {
    let dir = tempfile::tempdir().unwrap();
    let (_, a) = landed(
        dir.path(),
        &city::Slot::library("tools").unwrap(),
        "# kiln A\n",
    );
    let building = Address::parse("workshop").unwrap();
    std::fs::create_dir_all(dir.path().join(building.as_str())).unwrap();
    let (_, b) = landed(
        dir.path(),
        &city::Slot::building(&building, "tools").unwrap(),
        "# kiln B\n",
    );
    let mut ledger = open(dir.path());
    shelving(&mut ledger, a, remote("a/repo/kiln"));
    shelving(&mut ledger, b, remote("b/repo/kiln"));
    let (send, receive) = mpsc::channel();
    drop(send);
    let mut requests = BTreeMap::new();
    serve(
        dir.path(),
        ledger,
        &receive,
        &mut |request: &city::AuditRequest| {
            requests.insert(request.digest, request.source.clone());
            city::AuditReport::default()
        },
    )
    .unwrap();
    assert_eq!(
        requests,
        BTreeMap::from([(a, remote("a/repo/kiln")), (b, remote("b/repo/kiln"))])
    );
}
