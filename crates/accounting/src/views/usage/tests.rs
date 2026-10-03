// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A synthetic ledger folds to the usage table wire D33 describes, a skill
//! whose content changed asks for a new audit, an export writes each use
//! once.

use std::collections::BTreeSet;

use kernel::{Address, B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
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

/// Where a line sits: its sequence number, the run that wrote it, and
/// its moment.
#[derive(Debug, Clone, Copy)]
pub(super) struct At {
    seq: u64,
    by: u8,
    at: u64,
}

/// A line whose moment is its sequence number.
pub(super) fn now(seq: u64, by: u8) -> At {
    At { seq, by, at: seq }
}

fn later(seq: u64, by: u8, at: u64) -> At {
    At { seq, by, at }
}

pub(super) fn record(at: At, kind: EventKind, data: serde_json::Value) -> EventRecord {
    EventRecord::from_draft(
        EventDraft {
            run: run(at.by),
            t: TimeMs::new(at.at),
            who: "lab/parser".to_owned(),
            addr: Some(room()),
            kind,
            data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
            ig: false,
        },
        Seq::new(at.seq),
        B3Hash::digest(b"prev"),
    )
}

pub(super) fn started(seq: u64, by: u8, pins: &[(&str, &str)]) -> EventRecord {
    let skills: Vec<_> = pins
        .iter()
        .map(|(name, text)| json!({ "name": name, "hash": hash(text).to_string() }))
        .collect();
    record(
        now(seq, by),
        EventKind::RunStarted,
        json!({ "skills": skills }),
    )
}

pub(super) fn called(at: At, id: &str, tool: &str, args: serde_json::Value) -> EventRecord {
    let data = json!({ "id": id, "name": tool, "args": args });
    record(at, EventKind::ToolCalled, data)
}

fn connector(at: At, id: &str, tool: &str, label: &str) -> EventRecord {
    let effect = json!({ "connector": { "label": label } });
    let data = json!({ "id": id, "name": tool, "args": {}, "effect": effect });
    record(at, EventKind::ToolCalled, data)
}

fn answered(seq: u64, by: u8, id: &str) -> EventRecord {
    let data = json!({ "tool_use_id": id, "name": "read", "result": {} });
    record(now(seq, by), EventKind::ToolResult, data)
}

fn failed(seq: u64, by: u8, id: &str) -> EventRecord {
    let data = json!({ "tool_use_id": id, "name": "read", "error": {} });
    record(now(seq, by), EventKind::ToolResult, data)
}

fn used(at: At, part: &str, text: &str, outcome: wire::UseOutcome) -> wire::SkillUse {
    wire::SkillUse {
        run: run(at.by),
        resident: Some(room()),
        seq: Seq::new(at.seq),
        at: TimeMs::new(at.at),
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
        called(
            later(2, 1, 2),
            "c1",
            "describe",
            json!({ "name": "skill kiln" }),
        ),
        answered(3, 1, "c1"),
        called(
            later(4, 1, DAY + 4),
            "c2",
            "read",
            json!({ "path": "kiln" }),
        ),
        failed(5, 1, "c2"),
        called(
            later(6, 1, DAY + 6),
            "c3",
            "read",
            json!({ "path": "kiln/scripts/fire.py" }),
        ),
        called(later(7, 1, 7), "c4", "read", json!({ "path": "notes.md" })),
        started(8, 2, &[]),
        called(later(9, 2, 9), "c5", "read", json!({ "path": "kiln" })),
        connector(now(10, 1), "c6", "github_search", "github"),
        answered(11, 1, "c6"),
        called(
            later(12, 1, 12),
            "c7",
            "call",
            json!({ "name": "github_list", "args": {} }),
        ),
        called(
            later(13, 1, 13),
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
            author: wire::VersionAuthor::Unrecorded,
        }],
        uses: vec![
            used(later(2, 1, 2), "guide", "kiln v1", wire::UseOutcome::Ok),
            used(
                later(4, 1, DAY + 4),
                "SKILL.md",
                "kiln v1",
                wire::UseOutcome::Failed,
            ),
            used(
                later(6, 1, DAY + 6),
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

/// The usage pass hands every tool result to `runtime::ShellTally`, and
/// the shell table is its reading line for line (wire D48): shell lines
/// with a code counted per interpreter, failures by class, other arms
/// and other kinds passed over.
#[test]
fn the_usage_pass_answers_each_interpreters_calls_and_failures_by_class() {
    let result = |seq: u64, result: serde_json::Value| {
        record(
            now(seq, 1),
            EventKind::ToolResult,
            json!({ "tool_use_id": format!("c{seq}"), "name": "exec", "result": result }),
        )
    };
    let usage = Usage::fold([
        started(1, 1, &[]),
        result(
            2,
            json!({"arm": "shell", "interpreter": "pwsh", "exit_code": 1, "stdout": "", "stderr": "ParserError"}),
        ),
        result(
            3,
            json!({"arm": "shell", "interpreter": "pwsh", "exit_code": 0, "stdout": "ok", "stderr": ""}),
        ),
        result(
            4,
            json!({"arm": "shell", "exit_code": 9009, "stdout": "", "stderr": ""}),
        ),
        result(
            5,
            json!({"arm": "program", "exit_code": 2, "stdout": "", "stderr": ""}),
        ),
    ]);
    assert_eq!(
        usage.shells(),
        wire::ShellsAnswer {
            interpreters: vec![
                wire::ShellCalls {
                    interpreter: "pwsh".to_owned(),
                    calls: 2,
                    failures: [("syntax".to_owned(), 1)].into(),
                },
                wire::ShellCalls {
                    interpreter: "system".to_owned(),
                    calls: 1,
                    failures: [("command_not_found".to_owned(), 1)].into(),
                },
            ],
        }
    );
}
