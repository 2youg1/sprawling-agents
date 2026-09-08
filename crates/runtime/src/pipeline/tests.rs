// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use super::*;
use kernel::{ClockStampGranularity, Temporal, TimeMs};
use memory::Cas;

fn no_offload(cap: u64) -> PackContext<'static> {
    PackContext {
        cap_bytes: cap,
        stamp: None,
        net_notice: false,
        steer: None,
        reminder: None,
        offload: None,
        sieve: None,
    }
}

#[test]
fn small_results_pass_intact_with_zero_envelope_bytes() {
    let out = package(b"{\"ok\":true}", no_offload(4_096)).unwrap();
    assert_eq!(out.content, "{\"ok\":true}");
    assert!(out.events.is_empty(), "A18 spirit: no feature, no bytes");
}

#[test]
fn oversized_but_small_results_truncate_without_storing() {
    let big = "y".repeat(2_000);
    let out = package(big.as_bytes(), no_offload(100)).unwrap();
    assert!(out.content.starts_with("yyyy"));
    assert!(out.content.contains("[truncated: 1900 bytes]"));
    assert!(
        out.events.is_empty(),
        "invariant 3: lossy-without-restore stores nothing"
    );
}

#[test]
fn a_log_keeps_the_end_a_reader_needs_rather_than_the_beginning() {
    let mut log = String::new();
    for n in 0..400 {
        log.push_str(&format!(
            "2026-08-22T10:00:00Z line {n}
"
        ));
    }
    let out = package(log.as_bytes(), no_offload(400)).unwrap();
    assert!(
        out.content.contains("line 399"),
        "a log's last line is the one somebody is looking for: {}",
        out.content
    );
    assert!(
        out.content.contains("[truncated:"),
        "and it is never silent"
    );
}

#[test]
fn structured_content_is_never_cut_in_half_by_the_shortener() {
    // Compaction declines on JSON, so with no site to offload to the
    // byte cut is what is left - but it is said out loud, and the
    // marker is what tells a reader this is not a document.
    let json = format!("{{\"rows\":[{}]}}", "1,".repeat(500));
    let out = package(json.as_bytes(), no_offload(100)).unwrap();
    assert!(out.content.contains("[truncated:"));
    assert!(
        !out.content.contains("… (shortened) …"),
        "a shortened JSON object would still look parsable: {}",
        out.content
    );
}

#[test]
fn large_results_offload_first_and_account_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
    let env = dir.path().join("env");
    std::fs::create_dir_all(&env).unwrap();
    let big = vec![b'z'; 20_000];
    let out = package(
        &big,
        PackContext {
            cap_bytes: 1_024,
            stamp: None,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: Some(OffloadSite {
                cas: &mut cas,
                environment: &env,
            }),
            sieve: None,
        },
    )
    .unwrap();
    assert!(out.content.contains("[offloaded: total 20000 bytes"));
    assert_eq!(out.events.len(), 1);
    let event = serde_json::to_value(&out.events[0]).unwrap();
    assert!(event["original"].as_str().unwrap().starts_with("cas:b3-"));
    assert_eq!(event["len"], 20_000);
}

#[test]
fn an_exec_result_is_sieved_before_it_is_packaged() {
    let dir = tempfile::tempdir().unwrap();
    let mut cas = Cas::open(&dir.path().join("cas")).unwrap();
    let env = dir.path().join("env");
    std::fs::create_dir_all(&env).unwrap();
    let table = FilterTable::builtin();
    let mut history = SieveHistory::default();
    let noise = "   Compiling dep v0.1.0\n".repeat(200);
    let out = package(
        noise.as_bytes(),
        PackContext {
            cap_bytes: 16_384,
            stamp: None,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: Some(OffloadSite {
                cas: &mut cas,
                environment: &env,
            }),
            sieve: Some(SieveRequest {
                key: CommandKey::of(&kernel::ExecArm::Program {
                    path: "cargo".to_owned(),
                    args: vec!["check".to_owned()],
                }),
                exit_code: Some(0),
                table: &table,
                history: &mut history,
            }),
        },
    )
    .unwrap();
    assert!(
        out.content.starts_with("cargo check: clean, exit 0"),
        "{}",
        out.content
    );
    assert_eq!(out.events.len(), 1, "the tee is accounted");
    let event = serde_json::to_value(&out.events[0]).unwrap();
    assert_eq!(event["filter"], "cargo");
}

#[test]
fn the_three_attachments_ride_in_order_and_off_means_zero_bytes() {
    let mut gate = crate::clock::StampGate::new(ClockStampGranularity::Minute);
    let stamp = gate
        .observe(TimeMs::new(90_000), Temporal::Timestamped, &[])
        .unwrap();
    let out = package(
        b"done",
        PackContext {
            cap_bytes: 4_096,
            stamp,
            net_notice: true,
            steer: Some(("user".to_owned(), "wrap up".to_owned())),
            reminder: None,
            offload: None,
            sieve: None,
        },
    )
    .unwrap();
    let lines: Vec<&str> = out.content.lines().collect();
    assert_eq!(lines[0], "done");
    assert!(lines[1].starts_with("clock: utc 1970-01-01 00:01"));
    assert!(lines[2].starts_with("[net] "));
    assert_eq!(lines[3], "user: wrap up");
    // Off gate: byte-identical to the featureless envelope (A18).
    let mut off = crate::clock::StampGate::new(ClockStampGranularity::Off);
    let none = off
        .observe(TimeMs::new(90_000), Temporal::Timestamped, &[])
        .unwrap();
    let plain = package(
        b"done",
        PackContext {
            cap_bytes: 4_096,
            stamp: none,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: None,
            sieve: None,
        },
    )
    .unwrap();
    assert_eq!(plain.content, "done");
}

#[test]
fn an_overlong_attachment_is_cut_at_the_attachment_cap() {
    let noisy = "s".repeat(5_000);
    let out = package(
        b"ok",
        PackContext {
            cap_bytes: 4_096,
            stamp: None,
            net_notice: false,
            steer: Some(("user".to_owned(), noisy)),
            reminder: None,
            offload: None,
            sieve: None,
        },
    )
    .unwrap();
    assert!(out.content.len() < 2 + 1 + ENVELOPE_ATTACH_MAX_BYTES + 40);
    assert!(out.content.contains("[truncated: "));
}
