// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

fn four() -> (FrozenSegment, FrozenSegment, FrozenSegment, FrozenSegment) {
    (
        FrozenSegment::new(SegmentSlot::City, b"city bytes".to_vec()),
        FrozenSegment::new(SegmentSlot::Building, b"building bytes".to_vec()),
        FrozenSegment::new(SegmentSlot::Resident, b"resident bytes".to_vec()),
        FrozenSegment::new(SegmentSlot::Run, b"run bytes".to_vec()),
    )
}

#[test]
fn same_input_same_bytes_same_hashes() {
    let (c, b, r, run) = four();
    let (c2, b2, r2, run2) = four();
    let one = FrozenPrefix::assemble(c, b, r, run).unwrap();
    let two = FrozenPrefix::assemble(c2, b2, r2, run2).unwrap();
    assert_eq!(one.segment_hashes(), two.segment_hashes());
    assert_eq!(
        one.prompt_payload().unwrap(),
        two.prompt_payload().unwrap(),
        "A4: same input, same payload bytes"
    );
}

#[test]
fn slot_order_is_enforced() {
    let (c, b, r, run) = four();
    let err = FrozenPrefix::assemble(b, c, r, run).unwrap_err();
    assert_eq!(err.code(), &AxCode::InvalidArgs);
}

#[test]
fn payload_names_all_four_slots_in_order() {
    let (c, b, r, run) = four();
    let prefix = FrozenPrefix::assemble(c, b, r, run).unwrap();
    let json = serde_json::to_value(prefix.prompt_payload().unwrap()).unwrap();
    let slots: Vec<&str> = json["segments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["slot"].as_str().unwrap())
        .collect();
    assert_eq!(slots, ["city", "building", "resident", "run"]);
}

fn doc(addr: &str, body: &str) -> SourceDoc {
    SourceDoc {
        addr: Address::parse(addr).unwrap(),
        bytes: Some(body.as_bytes().to_vec()),
    }
}

fn plan() -> PrefixPlan {
    PrefixPlan {
        city: vec![doc("city.md", "be a good city")],
        building: vec![
            doc("b/building.md", "house rules"),
            doc("city.md", "duplicate of the city file"),
            SourceDoc {
                addr: Address::parse("b/missing.md").unwrap(),
                bytes: None,
            },
        ],
        resident: vec![doc("b/urbanite.md", "who i am")],
        run: vec![doc("b/room/job.md", "locator line")],
        caps: SegmentCaps::startup_default(),
    }
}

#[test]
fn built_prefix_is_deterministic_and_notes_account_everything() {
    let one = build_prefix(plan()).unwrap();
    let two = build_prefix(plan()).unwrap();
    assert_eq!(one.segment_hashes(), two.segment_hashes());
    let payload = serde_json::to_value(one.prompt_payload().unwrap()).unwrap();
    assert_eq!(
        payload,
        serde_json::to_value(two.prompt_payload().unwrap()).unwrap(),
        "A4: same plan, same payload bytes"
    );
    // Cross-slot dedup: city.md loads once, the second hit is noted.
    let building = &payload["segments"][1];
    let skipped = building["skipped"].as_array().unwrap();
    assert!(
        skipped
            .iter()
            .any(|s| s["addr"] == "city.md" && s["reason"] == "duplicate")
    );
    assert!(
        skipped
            .iter()
            .any(|s| s["addr"] == "b/missing.md" && s["reason"] == "unreadable")
    );
    // Breakpoints: all four segment edges, never more.
    assert_eq!(payload["breakpoints"].as_array().unwrap().len(), 4);
}

#[test]
fn oversized_documents_truncate_with_an_explicit_marker() {
    let mut p = plan();
    p.caps.run = 32;
    let long = "x".repeat(100);
    p.run = vec![SourceDoc {
        addr: Address::parse("b/room/job.md").unwrap(),
        bytes: Some(long.into_bytes()),
    }];
    let prefix = build_prefix(p).unwrap();
    let run_bytes = prefix.segments()[3].bytes().to_vec();
    let text = String::from_utf8(run_bytes).unwrap();
    assert!(text.len() <= 32, "cap holds including the marker");
    assert!(text.contains("[truncated: "), "never a silent tail drop");
    let payload = serde_json::to_value(prefix.prompt_payload().unwrap()).unwrap();
    let source = &payload["segments"][3]["sources"][0];
    assert_eq!(source["marker"], true);
    let kept = source["kept"].as_u64().unwrap();
    let dropped = source["dropped"].as_u64().unwrap();
    assert_eq!(kept + dropped, 100);
}

#[test]
fn multibyte_truncation_lands_on_a_char_boundary() {
    let mut p = plan();
    p.caps.city = 40;
    p.city = vec![SourceDoc {
        addr: Address::parse("city.md").unwrap(),
        bytes: Some(
            "\u{4e00}\u{4e8c}\u{4e09}\u{56db}\u{4e94}\u{516d}\u{4e03}\u{516b}\u{4e5d}\u{5341}"
                .repeat(3)
                .into_bytes(),
        ),
    }];
    let prefix = build_prefix(p).unwrap();
    assert!(String::from_utf8(prefix.segments()[0].bytes().to_vec()).is_ok());
}

#[test]
fn system_blocks_are_four_and_all_cache_marked() {
    let prefix = build_prefix(plan()).unwrap();
    let blocks = prefix.system_blocks().unwrap();
    assert_eq!(blocks.len(), 4);
    assert!(blocks.iter().all(|b| b.cache));
    assert_eq!(
        u32::try_from(blocks.iter().filter(|b| b.cache).count()).unwrap(),
        kernel::consts_external::CACHE_BREAKPOINTS_MAX
    );
}

#[test]
fn hash_changes_with_a_single_byte() {
    let one = FrozenSegment::new(SegmentSlot::City, b"abc".to_vec());
    let two = FrozenSegment::new(SegmentSlot::City, b"abd".to_vec());
    assert_ne!(one.hash(), two.hash());
}
