// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every use of a skill or a tool server as one row, written as JSONL or
//! as CSV (wire D33): the columns in one fixed order, an absent value
//! `null` in JSONL and empty in CSV, CSV quoted by RFC 4180 with `\r\n`
//! line ends on every platform.

/// The columns of a row, in the order every row writes them.
const COLUMNS: [&str; 10] = [
    "kind", "name", "server", "part", "run", "resident", "seq", "at_ms", "digest", "outcome",
];

/// One row: a value, or nothing, under each column.
pub(super) type Row = [Option<String>; 10];

/// Every read of every skill, one row each.
pub(super) fn skill_rows(answer: &wire::SkillUsageAnswer) -> Vec<Row> {
    answer
        .skills
        .iter()
        .flat_map(|line| {
            line.uses.iter().map(|used| {
                [
                    Some("skill".to_owned()),
                    Some(line.name.clone()),
                    None,
                    Some(used.part.clone()),
                    Some(used.run.to_string()),
                    used.resident.as_ref().map(|addr| addr.as_str().to_owned()),
                    Some(used.seq.value().to_string()),
                    Some(used.at.value().to_string()),
                    Some(used.digest.to_string()),
                    Some(outcome_word(used.outcome).to_owned()),
                ]
            })
        })
        .collect()
}

/// Every call of every server's tool, one row each.
pub(super) fn mcp_rows(answer: &wire::McpUsageAnswer) -> Vec<Row> {
    answer
        .servers
        .iter()
        .flat_map(|server| {
            server.tools.iter().flat_map(move |tool| {
                tool.uses.iter().map(move |used| {
                    [
                        Some("mcp".to_owned()),
                        Some(tool.tool.clone()),
                        server.server.clone(),
                        None,
                        Some(used.run.to_string()),
                        used.resident.as_ref().map(|addr| addr.as_str().to_owned()),
                        Some(used.seq.value().to_string()),
                        Some(used.at.value().to_string()),
                        None,
                        Some(outcome_word(used.outcome).to_owned()),
                    ]
                })
            })
        })
        .collect()
}

/// The rows as one text in `format`.
pub(super) fn write(rows: &[Row], format: wire::ExportFormat) -> String {
    match format {
        wire::ExportFormat::Jsonl => rows.iter().map(|row| jsonl_line(row) + "\n").collect(),
        wire::ExportFormat::Csv => std::iter::once(COLUMNS.join(","))
            .chain(rows.iter().map(|row| {
                row.iter()
                    .map(|value| value.as_deref().map(csv_field).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(",")
            }))
            .map(|line| line + "\r\n")
            .collect(),
    }
}

/// One object whose keys come in column order, which a map type would
/// sort; each key and value is escaped by `serde_json`.
fn jsonl_line(row: &Row) -> String {
    let fields: Vec<String> = COLUMNS
        .iter()
        .zip(row)
        .map(|(column, value)| {
            let value = value.as_deref().map_or(serde_json::Value::Null, |text| {
                serde_json::Value::String(text.to_owned())
            });
            format!(
                "{}:{value}",
                serde_json::Value::String((*column).to_owned())
            )
        })
        .collect();
    format!("{{{}}}", fields.join(","))
}

/// One CSV field, quoted when it holds a comma, a quote or a line end,
/// with each quote doubled (RFC 4180 section 2).
fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

fn outcome_word(outcome: wire::UseOutcome) -> &'static str {
    match outcome {
        wire::UseOutcome::Ok => "ok",
        wire::UseOutcome::Failed => "failed",
        wire::UseOutcome::Unknown => "unknown",
    }
}
