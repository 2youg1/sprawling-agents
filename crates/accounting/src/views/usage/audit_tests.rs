// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the usage fold makes of a held skill: who shelved each of its
//! versions, and whether a content whose digest changed since its audit
//! asks for a new one (wire D33).

use serde_json::json;

use kernel::{EventKind, EventRecord, Seq, TimeMs};

use super::Usage;
use super::tests::{At, fixture, hash, library, now, record, run, started};

fn shelving(at: At, skill: &str, text: &str) -> EventRecord {
    let data = json!({ "skill": skill, "digest": hash(text).to_string(),
                       "source": { "kind": "shipped" } });
    record(at, EventKind::SkillShelved, data)
}

/// Each version says who put it on the shelf, by ledger order alone
/// (wire D33): the city's `skill_shelved` line, a content the shelf got
/// outside the city's doors after the name was shelved, or a content
/// from before the city recorded shelving.
#[test]
fn each_version_says_who_shelved_it() {
    let ledger = [
        started(1, 1, &[("old", "old v1")]),
        shelving(now(2, 0), "kiln", "kiln v1"),
        started(3, 1, &[("kiln", "kiln v1")]),
        started(4, 2, &[("kiln", "kiln v2"), ("old", "old v1")]),
    ];
    let usage = Usage::fold(&ledger);
    let versions = |name: &str| usage.skills(&[], Some(name)).skills[0].versions.clone();
    let version = |seq: u64, by: u8, text: &str, author| wire::SkillVersion {
        digest: hash(text),
        seq: Seq::new(seq),
        at: TimeMs::new(seq),
        run: run(by),
        author,
    };
    let shipped = wire::VersionAuthor::Shelved {
        seq: Seq::new(2),
        from: kernel::event::record::ShelvedFrom::Shipped,
    };
    assert_eq!(
        versions("kiln"),
        vec![
            version(2, 0, "kiln v1", shipped),
            version(4, 2, "kiln v2", wire::VersionAuthor::OutsideShelf),
        ]
    );
    assert_eq!(
        versions("old"),
        vec![version(1, 1, "old v1", wire::VersionAuthor::Unrecorded)]
    );
}

#[test]
fn a_skill_whose_content_changed_is_asked_to_re_audit() {
    let mut ledger = fixture();
    ledger.push(record(
        now(14, 3),
        EventKind::SkillAudited,
        json!({ "skill": "kiln", "digest": hash("kiln v1").to_string(), "source": "skill_spector",
                "scanner": "skillspector 1", "verdict": "pass" }),
    ));
    let usage = Usage::fold(&ledger);
    let audit = |text: &str| {
        usage.skills(&[library("kiln", text)], Some("kiln")).skills[0].held[0]
            .audit
            .clone()
    };
    assert_eq!(
        audit("kiln v1"),
        wire::SkillAudit::Audited {
            verdict: kernel::event::record::AuditVerdict::Pass,
            at: Seq::new(14)
        }
    );
    assert_eq!(
        audit("kiln v2"),
        wire::SkillAudit::Stale {
            audited: hash("kiln v1")
        }
    );
}
