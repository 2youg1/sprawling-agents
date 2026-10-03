// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A synthetic ledger folds to the usage table wire D33 describes, a skill
//! whose content changed asks for a new audit, an export writes each use
//! once, and the fold holds the three properties
//! `crates/accounting/spec/Views/Usage.lean` proves of its model.

use std::collections::BTreeSet;

use kernel::{Address, B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use proptest::prelude::*;
use serde_json::json;

use super::{Shelved, Usage, export};

const DAY: u64 = 86_400_000;

fn run(byte: u8) -> RunId {
    RunId::from_bytes([byte; 16])
}

fn hash(text: &str) -> B3Hash {
    B3Hash::digest(text.as_bytes())
}

fn room() -> Address {
    Address::parse("lab/parser").unwrap()
}

fn record(seq: u64, by: u8, at: u64, kind: EventKind, data: serde_json::Value) -> EventRecord {
    EventRecord::from_draft(
        EventDraft {
            run: run(by),
            t: TimeMs::new(at),
            who: "lab/parser".to_owned(),
            addr: Some(room()),
            kind,
            data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
            ig: false,
        },
        Seq::new(seq),
        B3Hash::digest(b"prev"),
    )
}

fn started(seq: u64, by: u8, pins: &[(&str, &str)]) -> EventRecord {
    let skills: Vec<_> = pins
        .iter()
        .map(|(name, text)| json!({ "name": name, "hash": hash(text).to_string() }))
        .collect();
    record(
        seq,
        by,
        seq,
        EventKind::RunStarted,
        json!({ "skills": skills }),
    )
}

fn called(seq: u64, by: u8, at: u64, id: &str, tool: &str, args: serde_json::Value) -> EventRecord {
    record(
        seq,
        by,
        at,
        EventKind::ToolCalled,
        json!({ "id": id, "name": tool, "args": args }),
    )
}

fn connector(seq: u64, by: u8, id: &str, tool: &str, label: &str) -> EventRecord {
    let data = json!({ "id": id, "name": tool, "args": {}, "effect": { "connector": { "label": label } } });
    record(seq, by, seq, EventKind::ToolCalled, data)
}

fn answered(seq: u64, by: u8, id: &str) -> EventRecord {
    record(
        seq,
        by,
        seq,
        EventKind::ToolResult,
        json!({ "tool_use_id": id, "name": "read", "result": {} }),
    )
}

fn failed(seq: u64, by: u8, id: &str) -> EventRecord {
    record(
        seq,
        by,
        seq,
        EventKind::ToolResult,
        json!({ "tool_use_id": id, "name": "read", "error": {} }),
    )
}

fn used(
    seq: u64,
    by: u8,
    at: u64,
    part: &str,
    text: &str,
    outcome: wire::UseOutcome,
) -> wire::SkillUse {
    wire::SkillUse {
        run: run(by),
        resident: Some(room()),
        seq: Seq::new(seq),
        at: TimeMs::new(at),
        part: part.to_owned(),
        digest: hash(text),
        outcome,
    }
}

fn library(name: &str, text: &str) -> Shelved {
    Shelved {
        name: name.to_owned(),
        shelf: wire::SkillShelf::Library(Address::parse(&format!("hall/{name}.md")).unwrap()),
        digest: hash(text),
    }
}

/// The fixture: run 1 pins `kiln` and reads it three ways on two days,
/// plus a file that is no skill; run 2 pinned nothing and reads `kiln`,
/// which is not a use; three server calls, one through `call`, one to a
/// server nothing knows.
fn fixture() -> Vec<EventRecord> {
    vec![
        started(1, 1, &[("kiln", "kiln v1")]),
        called(2, 1, 2, "c1", "describe", json!({ "name": "skill kiln" })),
        answered(3, 1, "c1"),
        called(4, 1, DAY + 4, "c2", "read", json!({ "path": "kiln" })),
        failed(5, 1, "c2"),
        called(
            6,
            1,
            DAY + 6,
            "c3",
            "read",
            json!({ "path": "kiln/scripts/fire.py" }),
        ),
        called(7, 1, 7, "c4", "read", json!({ "path": "notes.md" })),
        started(8, 2, &[]),
        called(9, 2, 9, "c5", "read", json!({ "path": "kiln" })),
        connector(10, 1, "c6", "github_search", "github"),
        answered(11, 1, "c6"),
        called(
            12,
            1,
            12,
            "c7",
            "call",
            json!({ "name": "github_list", "args": {} }),
        ),
        called(
            13,
            1,
            13,
            "c8",
            "call",
            json!({ "name": "plan_finish", "args": {} }),
        ),
    ]
}

#[test]
fn a_fixture_ledger_folds_a_usage_table_per_skill_and_server() {
    let ledger = fixture();
    let usage = Usage::fold(&ledger);
    let shelved = [library("kiln", "kiln v1"), library("unused", "never read")];
    let day = |day: &str, count| wire::DayCount {
        day: day.to_owned(),
        count,
    };
    let kiln = wire::SkillUsageLine {
        name: "kiln".to_owned(),
        held: vec![wire::HeldSkill {
            shelf: shelved[0].shelf.clone(),
            digest: hash("kiln v1"),
            audit: wire::SkillAudit::Unaudited,
        }],
        versions: vec![wire::SkillVersion {
            digest: hash("kiln v1"),
            seq: Seq::new(1),
            at: TimeMs::new(1),
            run: run(1),
        }],
        uses: vec![
            used(2, 1, 2, "guide", "kiln v1", wire::UseOutcome::Ok),
            used(
                4,
                1,
                DAY + 4,
                "SKILL.md",
                "kiln v1",
                wire::UseOutcome::Failed,
            ),
            used(
                6,
                1,
                DAY + 6,
                "scripts/fire.py",
                "kiln v1",
                wire::UseOutcome::Unknown,
            ),
        ],
        per_day: vec![day("1970-01-01", 1), day("1970-01-02", 2)],
    };
    let unused = wire::SkillUsageLine {
        name: "unused".to_owned(),
        held: vec![wire::HeldSkill {
            shelf: shelved[1].shelf.clone(),
            digest: hash("never read"),
            audit: wire::SkillAudit::Unaudited,
        }],
        versions: vec![],
        uses: vec![],
        per_day: vec![],
    };
    assert_eq!(
        usage.skills(&shelved, None),
        wire::SkillUsageAnswer {
            skills: vec![kiln, unused.clone()]
        }
    );
    assert_eq!(
        usage.skills(&shelved, Some("unused")),
        wire::SkillUsageAnswer {
            skills: vec![unused]
        }
    );

    let call = |seq: u64, outcome| wire::McpUse {
        run: run(1),
        resident: Some(room()),
        seq: Seq::new(seq),
        at: TimeMs::new(seq),
        outcome,
    };
    let tool = |name: &str, uses: Vec<wire::McpUse>, days| wire::McpToolUsage {
        tool: name.to_owned(),
        uses,
        per_day: days,
    };
    let configured = BTreeSet::from(["github".to_owned(), "quiet".to_owned()]);
    assert_eq!(
        usage.mcp(&configured, None),
        wire::McpUsageAnswer {
            servers: vec![
                wire::McpServerUsage {
                    server: Some("github".to_owned()),
                    configured: true,
                    tools: vec![
                        tool(
                            "list",
                            vec![call(12, wire::UseOutcome::Unknown)],
                            vec![day("1970-01-01", 1)]
                        ),
                        tool(
                            "search",
                            vec![call(10, wire::UseOutcome::Ok)],
                            vec![day("1970-01-01", 1)]
                        ),
                    ],
                },
                wire::McpServerUsage {
                    server: Some("quiet".to_owned()),
                    configured: true,
                    tools: vec![]
                },
                wire::McpServerUsage {
                    server: None,
                    configured: false,
                    tools: vec![tool(
                        "plan_finish",
                        vec![call(13, wire::UseOutcome::Unknown)],
                        vec![day("1970-01-01", 1)]
                    )],
                },
            ],
        }
    );
}

#[test]
fn a_skill_whose_content_changed_is_asked_to_re_audit() {
    let mut ledger = fixture();
    ledger.push(record(
        14,
        3,
        14,
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

#[test]
fn an_export_writes_one_row_per_use_in_both_formats() {
    let answer = wire::McpUsageAnswer {
        servers: vec![wire::McpServerUsage {
            server: Some("github".to_owned()),
            configured: true,
            tools: vec![wire::McpToolUsage {
                tool: "say, \"hi\"".to_owned(),
                uses: vec![wire::McpUse {
                    run: run(1),
                    resident: None,
                    seq: Seq::new(3),
                    at: TimeMs::new(9),
                    outcome: wire::UseOutcome::Failed,
                }],
                per_day: vec![],
            }],
        }],
    };
    let rows = export::mcp_rows(&answer);
    let id = run(1).to_string();
    assert_eq!(
        export::write(&rows, wire::ExportFormat::Csv),
        format!(
            "kind,name,server,part,run,resident,seq,at_ms,digest,outcome\r\n\
             mcp,\"say, \"\"hi\"\"\",github,,{id},,3,9,,failed\r\n"
        )
    );
    assert_eq!(
        export::write(&rows, wire::ExportFormat::Jsonl),
        format!(
            "{{\"kind\":\"mcp\",\"name\":\"say, \\\"hi\\\"\",\"server\":\"github\",\"part\":null,\
             \"run\":\"{id}\",\"resident\":null,\"seq\":\"3\",\"at_ms\":\"9\",\"digest\":null,\
             \"outcome\":\"failed\"}}\n"
        )
    );
}

/// One line of the model in `Usage.lean`: a pin, a read, or another line.
#[derive(Debug, Clone)]
enum Line {
    Pinned { run: u8, skill: u8 },
    Read { run: u8, skill: u8 },
    Other,
}

fn lines() -> impl Strategy<Value = Vec<Line>> {
    let line = prop_oneof![
        (0u8..3, 0u8..3).prop_map(|(run, skill)| Line::Pinned { run, skill }),
        (0u8..3, 0u8..3).prop_map(|(run, skill)| Line::Read { run, skill }),
        Just(Line::Other),
    ];
    proptest::collection::vec(line, 0..24)
}

fn records(lines: &[Line]) -> Vec<EventRecord> {
    lines
        .iter()
        .zip(1u64..)
        .map(|(line, seq)| match line {
            Line::Pinned { run, skill } => started(seq, *run, &[(&format!("s{skill}"), "body")]),
            Line::Read { run, skill } => called(
                seq,
                *run,
                seq,
                &format!("c{seq}"),
                "read",
                json!({ "path": format!("s{skill}") }),
            ),
            Line::Other => record(seq, 0, seq, EventKind::RunFrozen, json!({})),
        })
        .collect()
}

proptest! {
    /// `a_read_the_run_did_not_pin_is_not_a_use` and `every_use_was_pinned`:
    /// a read counts exactly when its run pinned that skill before it.
    #[test]
    fn a_read_counts_exactly_when_its_run_pinned_the_skill(lines in lines()) {
        let usage = Usage::fold(&records(&lines));
        let mut pinned = BTreeSet::new();
        let mut expected = Vec::new();
        for (line, seq) in lines.iter().zip(1u64..) {
            match line {
                Line::Pinned { run, skill } => { pinned.insert((*run, *skill)); }
                Line::Read { run, skill } => if pinned.contains(&(*run, *skill)) { expected.push(seq); },
                Line::Other => {}
            }
        }
        let counted: Vec<u64> = usage.reads.iter().map(|read| read.used.seq.value()).collect();
        prop_assert_eq!(counted, expected);
    }

    /// `the_fold_only_appends`: folding more of the ledger never moves a
    /// use already counted.
    #[test]
    fn folding_more_only_appends_uses(lines in lines(), cut in 0usize..24) {
        let all = records(&lines);
        let cut = cut.min(all.len());
        let seqs = |usage: Usage| usage.reads.iter().map(|read| read.used.seq).collect::<Vec<_>>();
        let before = seqs(Usage::fold(&all[..cut]));
        let after = seqs(Usage::fold(&all));
        prop_assert_eq!(&after[..before.len()], &before[..]);
    }
}
