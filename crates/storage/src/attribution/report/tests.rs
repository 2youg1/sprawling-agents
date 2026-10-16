// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    reason = "test code"
)]

use super::*;
use kernel::{B3Hash, EventDraft, Payload, RunId, Seq, TimeMs};
use proptest::prelude::*;
#[cfg(test)]
fn record(
    run: RunId,
    who: &str,
    seq: u64,
    kind: EventKind,
    data: serde_json::Value,
) -> EventRecord {
    let map = data.as_object().cloned().unwrap_or_default();
    let draft = EventDraft {
        run,
        t: TimeMs::new(seq),
        who: who.to_owned(),
        addr: None,
        kind,
        data: Payload::new(map).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b""))
}
#[cfg(test)]
fn prompt(run: RunId, who: &str, seq: u64, lens: [u64; 4], window: u64) -> EventRecord {
    record(
        run,
        who,
        seq,
        EventKind::PromptAssembled,
        serde_json::json!({
            "segments": [
                { "slot": "city", "len": lens[0] },
                { "slot": "building", "len": lens[1] },
                { "slot": "resident", "len": lens[2] },
                { "slot": "run", "len": lens[3] },
            ],
            "window_bytes": window,
        }),
    )
}
fn sum(rows: &[(String, UsdMicros)]) -> u64 {
    rows.iter()
        .map(|(_, v)| v.get())
        .fold(0, u64::saturating_add)
}

#[test]
fn a20_every_cut_sums_to_the_billed_total() {
    let run = RunId::from_bytes([1u8; 16]);
    let mut attribution = Attribution::new();
    attribution
        .apply(&prompt(run, "alice", 0, [1000, 3000, 500, 77], 420))
        .unwrap();
    attribution
        .apply(&record(
            run,
            "alice",
            1,
            EventKind::ToolResult,
            serde_json::json!({ "name": "read", "bytes": 900 }),
        ))
        .unwrap();
    attribution
        .apply(&record(
            run,
            "alice",
            2,
            EventKind::ToolResult,
            serde_json::json!({ "name": "exec", "bytes": 100 }),
        ))
        .unwrap();
    attribution
        .apply(&record(
            run,
            "alice",
            3,
            EventKind::ModelReturned,
            serde_json::json!({ "billed_usd_micros": 999_983u64 }),
        ))
        .unwrap();

    let report = attribution.report();
    assert_eq!(report.total.get(), 999_983);
    assert_eq!(sum(&report.by_run), 999_983, "by_run");
    assert_eq!(sum(&report.by_actor), 999_983, "by_actor");
    assert_eq!(sum(&report.by_segment), 999_983, "by_segment");
    assert_eq!(sum(&report.by_tool), 999_983, "by_tool");
    assert_eq!(sum(&report.by_skill), 999_983, "by_skill");
    // The cuts are cuts, not copies: five segment buckets, two tools.
    assert_eq!(report.by_segment.len(), 5);
    assert_eq!(report.by_tool.len(), 2);
    // Weight order is respected: building is the largest segment.
    let biggest = report
        .by_segment
        .iter()
        .max_by_key(|(_, v)| v.get())
        .unwrap();
    assert_eq!(biggest.0, "building");
}

#[test]
fn a_call_with_no_basis_lands_in_the_honest_bucket() {
    let run = RunId::from_bytes([2u8; 16]);
    let mut attribution = Attribution::new();
    attribution
        .apply(&record(
            run,
            "bob",
            0,
            EventKind::ModelReturned,
            serde_json::json!({ "billed_usd_micros": 500u64 }),
        ))
        .unwrap();
    let report = attribution.report();
    assert_eq!(
        report.by_segment,
        vec![("unattributed".to_owned(), UsdMicros::new(500))]
    );
    assert_eq!(
        report.by_tool,
        vec![("no_tool".to_owned(), UsdMicros::new(500))]
    );
    assert_eq!(
        report.by_skill,
        vec![("no_skill".to_owned(), UsdMicros::new(500))]
    );
    assert_eq!(sum(&report.by_segment), 500);
}

#[test]
fn a_call_the_provider_never_billed_attributes_nothing() {
    let run = RunId::from_bytes([3u8; 16]);
    let mut attribution = Attribution::new();
    attribution
        .apply(&prompt(run, "carol", 0, [10, 10, 10, 10], 0))
        .unwrap();
    attribution
        .apply(&record(
            run,
            "carol",
            1,
            EventKind::ModelReturned,
            serde_json::json!({ "calls": 0 }),
        ))
        .unwrap();
    let report = attribution.report();
    assert_eq!(report.total.get(), 0);
    assert!(report.by_run.is_empty(), "no invented money");
    assert!(report.by_segment.is_empty());
}

#[test]
fn a_call_with_no_amount_is_counted_as_unpriced_with_its_tokens() {
    let run = RunId::from_bytes([4u8; 16]);
    let mut attribution = Attribution::new();
    let usage = serde_json::json!({
        "input_tokens": 1200, "output_tokens": 300,
        "cache_read_tokens": 40, "cache_write_tokens": 2
    });
    for (seq, data) in [
        serde_json::json!({ "usage": usage }),
        serde_json::json!({}),
        serde_json::json!({ "usage": usage, "billed_usd_micros": 700u64 }),
    ]
    .into_iter()
    .enumerate()
    {
        attribution
            .apply(&record(
                run,
                "dave",
                seq as u64,
                EventKind::ModelReturned,
                data,
            ))
            .unwrap();
    }
    let report = attribution.report();
    assert_eq!(
        report.unpriced,
        Unpriced {
            calls: 2,
            // The one usage-bearing call: 1200 input (the whole prompt,
            // cache parts included) + 300 output; the cache counts are
            // not added again. The other call reported no usage.
            tokens: 1500
        }
    );
    assert_eq!(report.total.get(), 700);
}

#[test]
fn every_renewal_is_counted_by_what_it_showed() {
    use kernel::event::record::CacheRenewed;
    use kernel::{CacheCount, ModelUsage, Tokens};
    let answered = |written: u64| CacheRenewed::Answered {
        usage: Some(ModelUsage {
            cache_write_tokens: CacheCount::Reported(Tokens::new(written)),
            ..ModelUsage::default()
        }),
        billed_usd_micros: Some(UsdMicros::new(900)),
    };
    let refused = CacheRenewed::Refused {
        refused: kernel::AxError::failure(
            kernel::AxCode::Provider,
            "renew a cached prefix",
            "rate limited",
        )
        .with_recovery("renewal is skipped until the next real request"),
    };
    let lines = [
        serde_json::to_value(answered(0)).unwrap(),
        serde_json::to_value(answered(4096)).unwrap(),
        serde_json::to_value(refused).unwrap(),
        serde_json::json!({ "usage": "not a usage" }),
    ];
    let mut attribution = Attribution::new();
    for (seq, data) in lines.into_iter().enumerate() {
        attribution
            .apply(&record(
                RunId::CITY,
                "city",
                seq as u64,
                EventKind::CacheRenewed,
                data,
            ))
            .unwrap();
    }
    let report = attribution.report();
    assert_eq!(
        (report.renewals, report.total.get()),
        (
            Renewals {
                billed: UsdMicros::new(1_800),
                read: 1,
                late: 1,
                unreported: 1,
                refused: 1,
            },
            1_800
        )
    );
}

/// A `cache_renewed` line as the accounting thread writes it: under
/// the city's run, by the owner, at the room it kept warm.
fn renewal(seq: u64, room: &str, renewed: &kernel::event::record::CacheRenewed) -> EventRecord {
    let draft = EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(seq),
        who: "owner".to_owned(),
        addr: Some(kernel::Address::parse(room).unwrap()),
        kind: EventKind::CacheRenewed,
        data: Payload::of(renewed).unwrap(),
        ig: false,
    };
    EventRecord::from_draft(draft, Seq::new(seq), B3Hash::digest(b""))
}

/// A renewal's bill is money the city spent: it joins the total and
/// every cut, under the room in `by_actor` and in the `keep_warm`
/// bucket of the other four, and a renewal no provider priced counts
/// as an unpriced call (storage D36).
#[test]
fn a_renewal_bill_joins_the_total_and_every_cut() {
    use kernel::event::record::CacheRenewed;
    use kernel::{ModelUsage, Tokens};
    let run = RunId::from_bytes([5u8; 16]);
    let room = "market/ito";
    let unpriced_usage = ModelUsage {
        input_tokens: Tokens::new(40),
        output_tokens: Tokens::new(1),
        ..ModelUsage::default()
    };
    let mut attribution = Attribution::new();
    attribution
        .apply(&record(
            run,
            room,
            0,
            EventKind::ModelReturned,
            serde_json::json!({ "billed_usd_micros": 1_000u64 }),
        ))
        .unwrap();
    for (seq, billed) in [(1u64, Some(UsdMicros::new(300))), (2, None)] {
        let renewed = CacheRenewed::Answered {
            usage: Some(unpriced_usage),
            billed_usd_micros: billed,
        };
        attribution.apply(&renewal(seq, room, &renewed)).unwrap();
    }
    let report = attribution.report();
    let row = |name: &str, micros: u64| (name.to_owned(), UsdMicros::new(micros));
    assert_eq!(
        (
            report.total,
            report.by_run,
            report.by_actor,
            report.by_segment,
            report.by_tool,
            report.by_skill,
            report.unpriced,
        ),
        (
            UsdMicros::new(1_300),
            vec![row(&run.to_string(), 1_000), row("keep_warm", 300)],
            vec![row(room, 1_300)],
            vec![row("keep_warm", 300), row(NO_SEGMENT, 1_000)],
            vec![row("keep_warm", 300), row(NO_TOOL, 1_000)],
            vec![row("keep_warm", 300), row(NO_SKILL, 1_000)],
            Unpriced {
                calls: 1,
                tokens: 41
            },
        )
    );
}

#[test]
fn tool_weights_belong_to_one_wave_only() {
    let run = RunId::from_bytes([4u8; 16]);
    let mut attribution = Attribution::new();
    for (seq, name) in [(0u64, "read"), (1, "edit")] {
        attribution
            .apply(&record(
                run,
                "dave",
                seq,
                EventKind::ToolResult,
                serde_json::json!({ "name": name, "bytes": 100 }),
            ))
            .unwrap();
    }
    attribution
        .apply(&record(
            run,
            "dave",
            2,
            EventKind::ModelReturned,
            serde_json::json!({ "billed_usd_micros": 1000u64 }),
        ))
        .unwrap();
    // Second call, no tools in between: it belongs to no_tool, not
    // to the previous wave's tools.
    attribution
        .apply(&record(
            run,
            "dave",
            3,
            EventKind::ModelReturned,
            serde_json::json!({ "billed_usd_micros": 400u64 }),
        ))
        .unwrap();
    let report = attribution.report();
    assert_eq!(sum(&report.by_tool), 1400);
    let tools: BTreeMap<String, u64> = report
        .by_tool
        .iter()
        .map(|(k, v)| (k.clone(), v.get()))
        .collect();
    assert_eq!(tools.get("read"), Some(&500));
    assert_eq!(tools.get("edit"), Some(&500));
    assert_eq!(tools.get("no_tool"), Some(&400));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// A20 as a property: whatever the amounts and whatever the
    /// weights, every cut still sums to the total exactly.
    #[test]
    fn a20_holds_for_arbitrary_amounts_and_weights(
        billed in prop::collection::vec(1u64..1_000_000_000, 1..8),
        lens in prop::array::uniform4(0u64..100_000),
        window in 0u64..100_000,
        tool_bytes in prop::collection::vec(0u64..10_000, 0..5),
    ) {
        let run = RunId::from_bytes([9u8; 16]);
        let mut attribution = Attribution::new();
        let mut expected: u64 = 0;
        let mut seq = 0u64;
        for amount in &billed {
            attribution.apply(&prompt(run, "p", seq, lens, window)).unwrap();
            seq = seq.saturating_add(1);
            for (i, bytes) in tool_bytes.iter().enumerate() {
                attribution
                    .apply(&record(
                        run,
                        "p",
                        seq,
                        EventKind::ToolResult,
                        serde_json::json!({ "name": format!("t{i}"), "bytes": bytes }),
                    ))
                    .unwrap();
                seq = seq.saturating_add(1);
            }
            attribution
                .apply(&record(
                    run,
                    "p",
                    seq,
                    EventKind::ModelReturned,
                    serde_json::json!({ "billed_usd_micros": amount }),
                ))
                .unwrap();
            seq = seq.saturating_add(1);
            expected = expected.saturating_add(*amount);
        }
        let report = attribution.report();
        prop_assert_eq!(report.total.get(), expected);
        prop_assert_eq!(sum(&report.by_run), expected);
        prop_assert_eq!(sum(&report.by_actor), expected);
        prop_assert_eq!(sum(&report.by_segment), expected);
        prop_assert_eq!(sum(&report.by_tool), expected);
        prop_assert_eq!(sum(&report.by_skill), expected);
    }
}

#[test]
fn a_call_is_split_by_its_own_runs_prompt_not_the_latest_in_the_ledger() {
    let first = RunId::from_bytes([3u8; 16]);
    let second = RunId::from_bytes([4u8; 16]);
    let mut attribution = Attribution::new();
    for row in [
        prompt(first, "alice", 0, [1000, 0, 0, 0], 0),
        prompt(second, "bob", 1, [0, 1000, 0, 0], 0),
        record(
            first,
            "alice",
            2,
            EventKind::ModelReturned,
            serde_json::json!({ "billed_usd_micros": 100u64 }),
        ),
    ] {
        attribution.apply(&row).unwrap();
    }
    let billed: Vec<(String, u64)> = attribution
        .report()
        .by_segment
        .into_iter()
        .map(|(slot, share)| (slot, share.get()))
        .filter(|(_, share)| *share > 0)
        .collect();
    assert_eq!(billed, vec![("city".to_owned(), 100)]);
}

/// A whole-ledger fold meets every run the city ever had; a frozen run
/// makes no further call, so its segment basis is not kept for it.
#[test]
fn a_frozen_run_leaves_no_segment_basis_behind() {
    let run = RunId::from_bytes([3; 16]);
    let mut attribution = Attribution::new();
    attribution
        .apply(&prompt(run, "resident", 1, [1, 1, 1, 1], 4))
        .unwrap();
    attribution
        .apply(&record(
            run,
            "resident",
            2,
            EventKind::RunFrozen,
            serde_json::json!({}),
        ))
        .unwrap();
    assert_eq!(attribution.segment_weights, BTreeMap::new());
}
