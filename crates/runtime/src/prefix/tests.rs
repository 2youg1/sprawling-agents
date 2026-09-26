// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

use std::num::NonZeroU32;

fn four() -> (FrozenSegment, FrozenSegment, FrozenSegment, FrozenSegment) {
    (
        FrozenSegment::new(SegmentSlot::City, b"city bytes".to_vec()),
        FrozenSegment::new(SegmentSlot::Building, b"building bytes".to_vec()),
        FrozenSegment::new(SegmentSlot::Resident, b"resident bytes".to_vec()),
        FrozenSegment::new(SegmentSlot::Run, b"run bytes".to_vec()),
    )
}

#[test]
fn the_startup_default_divides_the_token_budget_across_every_slot() {
    let (c, b, r, run) = four();
    let prefix = FrozenPrefix::assemble(c, b, r, run).unwrap();
    let slots = u64::try_from(prefix.segments().len()).unwrap();
    assert_eq!(
        slots,
        PREFIX_SLOTS.get(),
        "the divisor and the number of slots are the same four"
    );
    let whole = STARTUP_BUDGET_TOKENS * BYTES_PER_TOKEN;
    assert_eq!(
        SegmentCaps::startup_default(),
        SegmentCaps {
            city: whole / slots,
            building: whole / slots,
            resident: whole / slots,
            run: whole / slots,
        },
        "the caps are bytes and the budget they come from is tokens"
    );
}

#[test]
fn same_input_same_bytes_same_hashes() {
    let (c, b, r, run) = four();
    let (c2, b2, r2, run2) = four();
    let one = FrozenPrefix::assemble(c, b, r, run).unwrap();
    let two = FrozenPrefix::assemble(c2, b2, r2, run2).unwrap();
    assert_eq!(one.segment_hashes(), two.segment_hashes());
    assert_eq!(
        one.prompt_payload(&BreakpointPlan::for_conversation(&[]))
            .unwrap(),
        two.prompt_payload(&BreakpointPlan::for_conversation(&[]))
            .unwrap(),
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
    let json = serde_json::to_value(
        prefix
            .prompt_payload(&BreakpointPlan::for_conversation(&[]))
            .unwrap(),
    )
    .unwrap();
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
        producer: None,
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
                producer: None,
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
    let payload = serde_json::to_value(
        one.prompt_payload(&BreakpointPlan::for_conversation(&[]))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        payload,
        serde_json::to_value(
            two.prompt_payload(&BreakpointPlan::for_conversation(&[]))
                .unwrap()
        )
        .unwrap(),
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
    // Breakpoints: the edges the plan marks, and no tail on a request
    // with no conversation - the record names only what the wire carries.
    assert_eq!(
        payload["breakpoints"],
        serde_json::json!(["city", "building", "resident"])
    );
}

#[test]
fn oversized_documents_truncate_with_an_explicit_marker() {
    let mut p = plan();
    p.caps.run = 32;
    let long = "x".repeat(100);
    p.run = vec![SourceDoc {
        addr: Address::parse("b/room/job.md").unwrap(),
        bytes: Some(long.into_bytes()),
        producer: None,
    }];
    let prefix = build_prefix(p).unwrap();
    let run_bytes = prefix.segments()[3].bytes().to_vec();
    let text = String::from_utf8(run_bytes).unwrap();
    assert!(text.len() <= 32, "cap holds including the marker");
    assert!(text.contains("[truncated: "), "never a silent tail drop");
    let payload = serde_json::to_value(
        prefix
            .prompt_payload(&BreakpointPlan::for_conversation(&[]))
            .unwrap(),
    )
    .unwrap();
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
        producer: None,
    }];
    let prefix = build_prefix(p).unwrap();
    assert!(String::from_utf8(prefix.segments()[0].bytes().to_vec()).is_ok());
}

#[test]
fn system_blocks_mark_every_edge_but_the_one_the_tail_anchor_holds() {
    let prefix = build_prefix(plan()).unwrap();
    let blocks = prefix.system_blocks().unwrap();
    assert_eq!(blocks.len(), 4);
    let marked = u32::try_from(blocks.iter().filter(|b| b.cache).count()).unwrap();
    assert_eq!(
        marked.saturating_add(1),
        kernel::consts_external::CACHE_BREAKPOINTS_MAX,
        "three segment edges carry their breakpoint and the fourth is the tail anchor"
    );
    assert!(!blocks.last().unwrap().cache);
}

#[test]
fn hash_changes_with_a_single_byte() {
    let one = FrozenSegment::new(SegmentSlot::City, b"abc".to_vec());
    let two = FrozenSegment::new(SegmentSlot::City, b"abd".to_vec());
    assert_ne!(one.hash(), two.hash());
}

/// 每份压缩摘要在账本事件中标注生产模型与代数；非摘要的行不写这个键，
/// 来源不明的摘要写 `"unknown"`，于是「不是摘要」与「摘要但不知谁写的」在账上可分。
#[test]
fn a_summary_source_carries_its_producer_into_the_row() {
    let producer = SummaryProducer::Written {
        model: "model-a".to_owned(),
        generation: NonZeroU32::new(2).unwrap(),
    };
    let mut p = plan();
    p.city = vec![SourceDoc {
        addr: Address::parse("city/Summary.md").unwrap(),
        bytes: Some(b"# old work".to_vec()),
        producer: Some(producer),
    }];
    p.building = vec![SourceDoc {
        addr: Address::parse("lobby/RULES.md").unwrap(),
        bytes: Some(b"rules".to_vec()),
        producer: Some(SummaryProducer::Unknown),
    }];
    let built = build_prefix(p).unwrap();
    let json = serde_json::to_value(
        built
            .prompt_payload(&BreakpointPlan::for_conversation(&[]))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        json["segments"][0]["sources"][0]["producer"],
        serde_json::json!({ "written": { "model": "model-a", "generation": 2 } })
    );
    assert_eq!(
        json["segments"][1]["sources"][0]["producer"],
        serde_json::json!("unknown")
    );
    assert_eq!(
        json["segments"][2]["sources"][0].get("producer"),
        None,
        "非摘要的行不写这个键"
    );
}

/// 回放读的那条路：`build_prefix` 写出的载荷经 `Payload::read` 读回
/// `PromptAssembled`，逐源仍问得出生产者。写端的行断言与读端的行断言各自
/// 成立还不够——中间那次编码解码是它们唯一共享的动作，而回放走的正是它。
#[test]
fn the_payload_a_build_wrote_reads_back_with_its_producer_intact() {
    use kernel::event::record::PromptSource;

    let written = SummaryProducer::Written {
        model: "model-a".to_owned(),
        generation: NonZeroU32::new(3).unwrap(),
    };
    let mut p = plan();
    p.city = vec![SourceDoc {
        addr: Address::parse("city/Summary.md").unwrap(),
        bytes: Some(b"# old work".to_vec()),
        producer: Some(written.clone()),
    }];
    let payload = build_prefix(p)
        .unwrap()
        .prompt_payload(&BreakpointPlan::for_conversation(&[]))
        .unwrap();
    let read: PromptAssembled = payload.read().unwrap();
    let answered = |slot: usize| -> Vec<SummaryProducer> {
        read.segments[slot]
            .sources
            .iter()
            .map(PromptSource::producer)
            .collect()
    };
    assert_eq!(
        answered(0),
        vec![written],
        "换模型后回放仍答得出这份旧摘要是谁写的、第几代"
    );
    assert_eq!(
        answered(2),
        vec![SummaryProducer::Unknown],
        "没记生产者的行读回来是未知，不是回放当下跑的那个模型"
    );
}
