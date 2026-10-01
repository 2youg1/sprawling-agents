// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::fs;

use kernel::{EventDraft, EventKind, Payload, RunId, TimeMs};

use super::*;
use crate::jsonl::{CheckedLine, LineFault, ledger_segments_at, read_line};

fn draft(t: u64) -> EventDraft {
    EventDraft {
        run: RunId::CITY,
        t: TimeMs::new(t),
        who: "city".to_string(),
        addr: None,
        kind: EventKind::GateChecked,
        data: Payload::empty(),
        ig: false,
    }
}

/// Four lines, one per segment, so the audit has to cross segments.
fn four_segment_ledger(dir: &Path) -> JsonlLedger {
    let (mut ledger, _) = JsonlLedger::open(dir, TimeMs::new(0)).unwrap();
    ledger.set_roll_bytes_for_test(1);
    for t in 0..4 {
        ledger.append_all(vec![draft(t)]).unwrap();
    }
    ledger
}

#[test]
fn an_intact_chain_is_whole_across_segments_and_a_torn_tail_is_not_a_line() {
    let tmp = tempfile::tempdir().unwrap();
    let _ledger = four_segment_ledger(tmp.path());
    let last = ledger_segments_at(tmp.path()).unwrap().pop().unwrap();
    let mut bytes = fs::read(&last).unwrap();
    bytes.extend_from_slice(b"{\"v\":1,\"torn");
    fs::write(&last, bytes).unwrap();

    assert_eq!(
        audit_chain(tmp.path()).unwrap(),
        ChainAudit::Whole { lines: 4 }
    );
}

#[test]
fn an_early_line_changed_in_place_breaks_the_chain_at_the_line_after_it() {
    let tmp = tempfile::tempdir().unwrap();
    let _ledger = four_segment_ledger(tmp.path());
    let first = ledger_segments_at(tmp.path()).unwrap().remove(0);
    let tampered = fs::read_to_string(&first)
        .unwrap()
        .replace("\"who\":\"city\"", "\"who\":\"town\"");
    fs::write(&first, tampered).unwrap();

    assert_eq!(
        audit_chain(tmp.path()).unwrap(),
        ChainAudit::Broken(LineFault::ChainBreak.into_ax(2))
    );
}

#[test]
fn a_tripped_halt_refuses_the_next_write_with_the_audits_own_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ledger = four_segment_ledger(tmp.path());
    let halt = ChainHalt::default();
    ledger.halt_on(halt.clone());
    let reason = LineFault::ChainBreak.into_ax(2);
    halt.trip(reason.clone());
    halt.trip(LineFault::ChainBreak.into_ax(3));

    let refused = ledger.append_all(vec![draft(9)]).unwrap_err().into_ax();

    assert_eq!((refused, halt.reason()), (reason.clone(), Some(reason)));
    assert_eq!(ledger.read_raw_lines().unwrap().len(), 4);
}

/// A ledger of one line per segment, `lines` long, under `dir/ledger`,
/// and a writable home for its proof records under `dir/records`.
fn ledger_of(dir: &Path, lines: u64) -> (JsonlLedger, ProofRecords) {
    let (mut ledger, _) = JsonlLedger::open(&dir.join("ledger"), TimeMs::new(0)).unwrap();
    ledger.set_roll_bytes_for_test(1);
    for t in 0..lines {
        ledger.append_all(vec![draft(t)]).unwrap();
    }
    let records = ledger.proof_records(&dir.join("records"));
    (ledger, records)
}

fn bytes_on_disk(ledger_dir: &Path) -> u64 {
    ledger_segments_at(ledger_dir)
        .unwrap()
        .iter()
        .map(|segment| fs::metadata(segment).unwrap().len())
        .sum()
}

/// What makes a reopened city cheap to prove: every segment the first
/// proof checked is taken from its record, so the lines checked the
/// second time are the lines written since, however long the history.
#[test]
fn a_second_proof_checks_only_the_lines_written_since_the_first() {
    let proved = |history: u64| {
        let tmp = tempfile::tempdir().unwrap();
        let (mut ledger, records) = ledger_of(tmp.path(), history);
        let ledger_dir = tmp.path().join("ledger");
        prove_chain(&ledger_dir, &records).unwrap();
        ledger.append_all(vec![draft(90), draft(91)]).unwrap();
        (
            prove_chain(&ledger_dir, &records).unwrap(),
            bytes_on_disk(&ledger_dir),
        )
    };
    let expected = |history: u64, total: u64| Proven {
        audit: ChainAudit::Whole { lines: history + 2 },
        counted: ProofCount {
            lines_checked: 2,
            segments_by_digest: history,
            bytes_read: total,
            bytes_hashed: total,
            waves: 1,
        },
        unkept: None,
    };

    let ((short, short_total), (long, long_total)) = (proved(3), proved(6));

    assert_eq!(
        (short, long),
        (expected(3, short_total), expected(6, long_total))
    );
}

/// A record stands in for its segment only while the chain still enters
/// that segment where the record says: a segment rewritten in place, or
/// one deleted from the middle with its record left behind, is refused
/// exactly as a walk with no records refuses it.
#[test]
fn a_record_is_reused_only_where_the_chain_still_links_to_it() {
    let reproved = |damage: &dyn Fn(&Path, &Path)| reproved_with(4, damage).0.audit;
    let rewritten = reproved(&|ledger_dir, _| {
        let first = ledger_segments_at(ledger_dir).unwrap().remove(0);
        let tampered = fs::read_to_string(&first)
            .unwrap()
            .replace("\"who\":\"city\"", "\"who\":\"town\"");
        fs::write(&first, tampered).unwrap();
    });
    let spliced = reproved(&|ledger_dir, _| {
        let third = ledger_segments_at(ledger_dir).unwrap().remove(2);
        fs::remove_file(third).unwrap();
    });

    assert_eq!(
        (rewritten, spliced),
        (
            ChainAudit::Broken(LineFault::ChainBreak.into_ax(2)),
            ChainAudit::Broken(LineFault::ChainBreak.into_ax(3)),
        )
    );
}

/// A holder without the writer lock reads records and never writes one,
/// so a read-only command leaves the city's disk as it found it.
#[test]
fn a_read_only_proof_writes_no_record() {
    let tmp = tempfile::tempdir().unwrap();
    let (_ledger, _) = ledger_of(tmp.path(), 3);
    let records_dir = tmp.path().join("records");
    let records = ProofRecords::read_only(&records_dir);

    let proven = prove_chain(&tmp.path().join("ledger"), &records).unwrap();

    assert_eq!(
        (
            proven.audit,
            proven.counted.lines_checked,
            records_dir.exists()
        ),
        (ChainAudit::Whole { lines: 3 }, 3, false)
    );
}

/// A served writer takes no line before the proof of its history, takes
/// lines once the chain is proved whole, and keeps the first verdict.
#[test]
fn a_writer_awaiting_proof_refuses_until_the_chain_is_proved() {
    let tmp = tempfile::tempdir().unwrap();
    let mut ledger = four_segment_ledger(tmp.path());
    let halt = ChainHalt::awaiting_proof();
    ledger.halt_on(halt.clone());

    let before = ledger
        .append_all(vec![draft(9)])
        .map_err(|refused| *refused.into_ax().code());
    halt.prove();
    ledger.await_verdict();
    let after = ledger.append_all(vec![draft(10)]).map(|_| ());
    halt.trip(LineFault::ChainBreak.into_ax(2));
    let later = ledger.append_all(vec![draft(11)]).map(|_| ());

    assert_eq!(
        (
            before.err(),
            after.is_ok(),
            later.is_ok(),
            halt.proved(),
            halt.reason()
        ),
        (
            Some(kernel::AxCode::HistoryUnproven),
            true,
            true,
            true,
            None
        )
    );
    assert_eq!(ledger.read_raw_lines().unwrap().len(), 6);
}

/// The verdict on one line of a fixed fixture, as a word: which arm of
/// the check it took, not the words of its reason.
fn verdict(checked: Result<CheckedLine, LineFault>) -> String {
    match checked {
        Ok(CheckedLine::Known(record)) => format!("known {}", record.seq().value()),
        Ok(CheckedLine::IgnoredUnknown(seq)) => format!("ignored {}", seq.value()),
        Err(LineFault::NotALine(_)) => "not a line".to_owned(),
        Err(LineFault::VersionAhead(v)) => format!("ahead {v}"),
        Err(LineFault::NotAVersion(v)) => format!("not a version {v}"),
        Err(LineFault::ChainBreak) => "chain break".to_owned(),
        Err(LineFault::SeqGap { found, expected }) => {
            format!("gap {} {}", found.value(), expected.value())
        }
        Err(LineFault::UnknownKind(kind)) => format!("unknown {kind}"),
        Err(LineFault::NotCanonical(_)) => "not canonical".to_owned(),
        Err(LineFault::SeqExhausted(_)) => "exhausted".to_owned(),
    }
}

/// The rule version moves with the rule: a change to what the per-line
/// check accepts - the event table, the canonical echo, the envelope -
/// changes a line or a verdict here, and this test names the value
/// `LINE_CHECK_RULES` must take (storage-SPEC 8-30).
#[test]
fn line_check_rules_pin_the_verdicts_of_a_fixed_fixture() {
    let mut prev = kernel::GENESIS_PREV;
    let mut lines: Vec<Vec<u8>> = Vec::new();
    for (at, kind) in [
        EventKind::CityInitialized,
        EventKind::RunStarted,
        EventKind::GateChecked,
    ]
    .into_iter()
    .enumerate()
    {
        let line = kernel::EventRecord::from_draft(
            EventDraft {
                kind,
                ..draft(1_000)
            },
            kernel::Seq::new(u64::try_from(at).unwrap()),
            prev,
        )
        .canonical_line()
        .unwrap();
        prev = kernel::ledger::chain_hash(&line);
        lines.push(line);
    }
    let mut check = LineCheck::at_genesis();
    let mut verdicts: Vec<String> = lines
        .iter()
        .map(|line| verdict(check.advance(line)))
        .collect();
    let first = String::from_utf8(lines[0].clone()).unwrap();
    let future = first.replace("\"city_initialized\"", "\"kind_from_the_future\"");
    let odd: Vec<Vec<u8>> = [
        future.clone(),
        format!("{},\"ig\":true}}", future.strip_suffix('}').unwrap()),
        first.replacen('{', "{ ", 1),
        first.replacen("{\"v\":", "{\"v\":9", 1),
    ]
    .into_iter()
    .map(String::into_bytes)
    .collect();
    verdicts.extend(odd.iter().map(|line| verdict(read_line(line))));
    let fixture: Vec<u8> = lines
        .iter()
        .chain(odd.iter())
        .flat_map(|line| line.iter().copied().chain(*b"\n"))
        .chain(verdicts.join("\n").into_bytes())
        .collect();

    let digest = kernel::B3Hash::digest(&fixture).to_string();
    assert_eq!(
        crate::verified_prefix::LINE_CHECK_RULES,
        format!("line-check-{}", digest.get(..16).unwrap()),
        "the per-line check changed what it accepts: set LINE_CHECK_RULES to this value"
    );
}

/// A ledger reopened after its history was proved checks only the lines
/// written since the proof, however long its last segment has grown: the
/// opening takes the last segment's record through the judgement the
/// proof uses, and reads and hashes the prefix once (storage-SPEC 8-34).
#[test]
fn a_reopened_ledger_checks_only_what_its_last_record_has_not_proved() {
    let reopened = |history: u64| {
        let tmp = tempfile::tempdir().unwrap();
        let (dir, kept) = (tmp.path().join("ledger"), tmp.path().join("records"));
        let (mut ledger, _) = JsonlLedger::open(&dir, TimeMs::new(0)).unwrap();
        ledger
            .append_all((0..history).map(draft).collect())
            .unwrap();
        prove_chain(&dir, &ledger.proof_records(&kept)).unwrap();
        let last = ledger_segments_at(&dir).unwrap().pop().unwrap();
        let proved = fs::metadata(&last).unwrap().len();
        ledger.append_all(vec![draft(90), draft(91)]).unwrap();
        drop(ledger);
        let records = ProofRecords::read_only(&kept);
        let (ledger, report) = JsonlLedger::open_reusing(&dir, TimeMs::new(99), &records).unwrap();
        assert_eq!(
            u64::try_from(ledger.read_raw_lines().unwrap().len()).unwrap(),
            history + 2
        );
        let expected = ProofCount {
            lines_checked: 2,
            segments_by_digest: 1,
            bytes_read: fs::metadata(&last).unwrap().len(),
            bytes_hashed: proved,
            waves: 0,
        };
        (report.counted, expected)
    };
    for history in [40, 80] {
        let (counted, expected) = reopened(history);
        assert_eq!(counted, expected, "a history of {history} lines");
    }
}

/// A second proof of `segments` one-line segments, after `damage`
/// touched the ledger and the records the first proof wrote, with the
/// bytes the segments hold.
fn reproved_with(segments: u64, damage: &dyn Fn(&Path, &Path)) -> (Proven, u64) {
    let tmp = tempfile::tempdir().unwrap();
    let (_ledger, records) = ledger_of(tmp.path(), segments);
    let ledger_dir = tmp.path().join("ledger");
    prove_chain(&ledger_dir, &records).unwrap();
    damage(&ledger_dir, &tmp.path().join("records"));
    (
        prove_chain(&ledger_dir, &records).unwrap(),
        bytes_on_disk(&ledger_dir),
    )
}

/// The segments are read in waves of `PROOF_WAVE`, so the number of
/// waves follows the number of segments and nothing else: not the cores
/// of the machine, and not which thread of a wave finished first. A
/// segment whose record is gone is checked line by line while the rest of
/// its wave stands by their records.
#[test]
fn a_proof_reads_in_waves_and_checks_alone_a_segment_without_its_record() {
    let expected = |segments: u64, checked: u64, total: u64| Proven {
        audit: ChainAudit::Whole { lines: segments },
        counted: ProofCount {
            lines_checked: checked,
            segments_by_digest: segments - checked,
            bytes_read: total,
            bytes_hashed: total,
            waves: segments.div_ceil(8),
        },
        unkept: None,
    };
    let unrecorded = |ledger_dir: &Path, records_dir: &Path| {
        let fifth = ledger_segments_at(ledger_dir).unwrap().remove(4);
        let name = fifth.file_name().unwrap().to_str().unwrap().to_owned();
        fs::remove_file(records_dir.join(format!("{name}.proof"))).unwrap();
    };

    let ((short, short_total), (long, long_total), (gap, gap_total)) = (
        reproved_with(9, &|_, _| {}),
        reproved_with(18, &|_, _| {}),
        reproved_with(9, &unrecorded),
    );

    assert_eq!(
        (short, long, gap),
        (
            expected(9, 0, short_total),
            expected(18, 0, long_total),
            expected(9, 1, gap_total)
        )
    );
}
