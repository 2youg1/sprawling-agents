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
    shelving(&mut ledger, a, remote("other/repo/kiln"));
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

fn records(root: &Path) -> Vec<EventRecord> {
    let mut records = Vec::new();
    runtime::replay::fold_ledger_dir(&kernel::layout::CityLayout::new(root).ledger(), |record| {
        records.push(record.clone());
        Ok(())
    })
    .unwrap();
    records
}

fn local_report(request: &city::AuditRequest, verdict: AuditVerdict) -> city::AuditReport {
    city::AuditReport {
        records: vec![SkillAudited {
            local_only_reason: None,
            skill: request.skill.clone(),
            digest: request.digest,
            source: kernel::event::record::AuditSource::SkillSpector,
            scanner: "scripted scanner".into(),
            verdict,
            risk: None,
            audited_at: None,
            link: None,
        }],
        failures: vec![],
    }
}

#[test]
fn background_unknown_digest_scans_locally_and_records_why() {
    let dir = tempfile::tempdir().unwrap();
    let (path, old) = landed(
        dir.path(),
        &city::Slot::library("tools").unwrap(),
        "# kiln\n",
    );
    let mut ledger = open(dir.path());
    shelving(&mut ledger, old, remote("a/repo/kiln"));
    std::fs::write(&path, "# edited kiln\n").unwrap();
    let current = city::skill_digest(&path).unwrap();
    let (send, receive) = mpsc::channel();
    drop(send);
    let mut scans = Vec::new();
    serve(
        dir.path(),
        ledger,
        &receive,
        &mut |request: &city::AuditRequest| {
            city::audit_skill(
                request,
                &mut |_| panic!("unknown digest must never reach remote"),
                city::LocalScanner::Available(&mut |path| {
                    scans.push(path.to_path_buf());
                    Ok(city::ScannerAudit {
                        version: "scanner 1".into(),
                        code: Some(0),
                        body: b"{}".to_vec(),
                    })
                }),
            )
        },
    )
    .unwrap();
    let written = records(dir.path());
    let last = written.last().unwrap();
    assert_eq!(scans, vec![path]);
    assert_eq!(last.data().read::<SkillAudited>().unwrap().digest, current);
    assert_eq!(
        last.data().as_map().get("local_only_reason").unwrap(),
        UNREGISTERED_CONTENT
    );
}

#[test]
fn background_startup_notification_and_duplicate_claims_use_one_attempt() {
    let dir = tempfile::tempdir().unwrap();
    let (path, a) = landed(
        dir.path(),
        &city::Slot::library("tools").unwrap(),
        "# kiln A\n",
    );
    let mut ledger = open(dir.path());
    shelving(&mut ledger, a, ShelvedFrom::Path);
    let (send, receive) = mpsc::channel();
    for notice in records(dir.path()) {
        send.send(notice).unwrap();
    }
    let mut calls = Vec::new();
    let mut sender = Some(send);
    serve(
        dir.path(),
        ledger,
        &receive,
        &mut |request: &city::AuditRequest| {
            calls.push(request.digest);
            if request.digest == a {
                std::fs::write(&path, "# kiln B\n").unwrap();
                let b = city::skill_digest(&path).unwrap();
                let notice = EventRecord::from_draft(
                    draft(
                        EventKind::SkillShelved,
                        Payload::of(&SkillShelved {
                            skill: "kiln".into(),
                            digest: b,
                            source: ShelvedFrom::Path,
                        })
                        .unwrap(),
                    ),
                    Seq::new(100),
                    kernel::ledger::GENESIS_PREV,
                );
                sender.as_ref().unwrap().send(notice.clone()).unwrap();
                sender.as_ref().unwrap().send(notice).unwrap();
            } else {
                drop(sender.take());
            }
            local_report(request, AuditVerdict::Unreachable)
        },
    )
    .unwrap();
    let b = city::skill_digest(&path).unwrap();
    assert_eq!(calls, vec![a, b]);
    assert_eq!(
        records(dir.path())
            .iter()
            .filter(|r| r.kind() == EventKind::SkillAudited)
            .count(),
        2
    );
}

#[test]
fn background_success_is_not_reaudited_but_unreachable_retries_next_process() {
    for verdict in [
        AuditVerdict::Pass,
        AuditVerdict::Warn,
        AuditVerdict::Fail,
        AuditVerdict::Unreachable,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_, digest) = landed(
            dir.path(),
            &city::Slot::library("tools").unwrap(),
            "# kiln\n",
        );
        let mut calls = Vec::new();
        for _ in 0..2 {
            let (send, receive) = mpsc::channel();
            drop(send);
            serve(
                dir.path(),
                open(dir.path()),
                &receive,
                &mut |request: &city::AuditRequest| {
                    calls.push(request.digest);
                    local_report(request, verdict)
                },
            )
            .unwrap();
        }
        assert_eq!(
            calls,
            if verdict == AuditVerdict::Unreachable {
                vec![digest, digest]
            } else {
                vec![digest]
            }
        );
    }
}

struct RefusingLedger {
    writes: usize,
}
impl Ledger for &mut RefusingLedger {
    fn append(&mut self, _: EventDraft) -> Result<kernel::EventRef, AxError> {
        self.writes = self.writes.checked_add(1).unwrap();
        Err(AxError::failure(
            AxCode::StorageFatal,
            "append an audit",
            "scripted writer failure",
        )
        .with_recovery("restart the writer"))
    }
}

#[test]
fn background_write_failure_returns_without_consuming_notification() {
    let dir = tempfile::tempdir().unwrap();
    landed(
        dir.path(),
        &city::Slot::library("tools").unwrap(),
        "# kiln\n",
    );
    drop(open(dir.path()));
    let (send, receive) = mpsc::channel();
    let notice = EventRecord::from_draft(
        draft(EventKind::RunStarted, Payload::empty()),
        Seq::new(100),
        kernel::ledger::GENESIS_PREV,
    );
    send.send(notice.clone()).unwrap();
    let mut ledger = RefusingLedger { writes: 0 };
    let mut calls = 0usize;
    let result = serve(
        dir.path(),
        &mut ledger,
        &receive,
        &mut |request: &city::AuditRequest| {
            calls = calls.checked_add(1).unwrap();
            local_report(request, AuditVerdict::Pass)
        },
    );
    assert_eq!(
        (result.unwrap_err().code(), ledger.writes, calls),
        (&AxCode::StorageFatal, 1, 1)
    );
    assert_eq!(receive.try_recv().unwrap(), notice);
}

#[test]
fn background_program_version_failure_records_unreachable_without_scan() {
    let dir = tempfile::tempdir().unwrap();
    let (path, digest) = landed(
        dir.path(),
        &city::Slot::library("tools").unwrap(),
        "# kiln\n",
    );
    let script = tempfile::tempdir().unwrap();
    let marker = script.path().join("scanned");

    #[cfg(windows)]
    let program = {
        let program = script.path().join("skillspector.cmd");
        std::fs::write(&program, format!("@echo off\r\nif \"%1\"==\"--version\" (\r\necho failed version\r\nexit /b 7\r\n)\r\necho scan > \"{}\"\r\nexit /b 0\r\n", marker.display())).unwrap();
        program
    };
    #[cfg(not(windows))]
    let program = {
        use std::os::unix::fs::PermissionsExt;
        let program = script.path().join("skillspector");
        std::fs::write(&program, format!("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\necho failed version\nexit 7\nfi\necho scan > '{}'\n", marker.display())).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        program
    };
    let (send, receive) = mpsc::channel();
    drop(send);
    serve(
        dir.path(),
        open(dir.path()),
        &receive,
        &mut |request: &city::AuditRequest| {
            assert_eq!((&request.path, request.digest), (&path, digest));
            let report = city::audit_skill(
                request,
                &mut |_| panic!("local only"),
                city::LocalScanner::Available(&mut |path| clients::scan(&program, path)),
            );
            assert!(
                report
                    .failures
                    .first()
                    .unwrap()
                    .subject()
                    .contains("Some(7)")
            );
            report
        },
    )
    .unwrap();
    let audits = records(dir.path())
        .into_iter()
        .filter(|r| r.kind() == EventKind::SkillAudited)
        .map(|r| r.data().read::<SkillAudited>().unwrap().verdict)
        .collect::<Vec<_>>();
    assert_eq!(audits, vec![AuditVerdict::Unreachable]);
    assert!(!marker.exists());
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(24))]
    #[test]
    fn background_claims_keep_nodup_for_notification_traces(count in 0usize..32) {
        let dir = tempfile::tempdir().unwrap();
        landed(dir.path(), &city::Slot::library("tools").unwrap(), "# kiln
");
        let ledger = open(dir.path());
        let (send, receive) = mpsc::channel();
        for index in 0..count {
            send.send(EventRecord::from_draft(draft(EventKind::RunStarted, Payload::empty()),
                Seq::new(u64::try_from(index).unwrap()), kernel::ledger::GENESIS_PREV)).unwrap();
        }
        drop(send);
        let mut calls = 0usize;
        serve(dir.path(), ledger, &receive, &mut |request: &city::AuditRequest| {
            calls = calls.checked_add(1).unwrap();
            local_report(request, AuditVerdict::Unreachable)
        }).unwrap();
        proptest::prop_assert_eq!(calls, 1);
    }
}
