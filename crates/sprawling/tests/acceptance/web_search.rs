// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `web_search` tool (`crates/accounting/spec/Connectors.lean` §8-35): its
//! catalogue episode, against a search supplier on a loopback port that
//! the city's `[search]` table selects, so the catalogue never reaches a
//! service off this machine.

use std::io::{BufRead as _, Read as _, Write as _};
use std::path::Path;

use serde_json::json;

use kernel::event::record::ToolAnswer;

use crate::episodes::{Episode, Observed};
use crate::script::Step;

/// What the loopback supplier finds for every query.
const FOUND: &str = "the kiln fires at 1280 degrees";

/// Selects a supplier on a loopback port in the city's own `[search]`
/// table: one anonymous account, its remote tool taking `q`.
pub(crate) fn supply(city_root: &Path) {
    let id = kernel::ServerLabel::parse("loopback").unwrap();
    ::city::write_search(
        city_root,
        &kernel::config::SearchConfiguration::Custom {
            selected: id.clone(),
            suppliers: vec![kernel::config::SearchSupplier {
                id,
                url: serve(),
                remote: "find".to_owned(),
                query_field: "q".to_owned(),
                objective_field: None,
                count_field: None,
                accounts: vec![kernel::event::record::ProviderAccount {
                    id: kernel::ServerLabel::parse("anonymous").unwrap(),
                    reference: None,
                    header: None,
                }],
            }],
        },
    )
    .unwrap();
}

/// A supplier that answers every MCP request it is sent, for as long as
/// the test process lives.
fn serve() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut reader = std::io::BufReader::new(stream.unwrap());
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let result = match request["method"].as_str() {
                Some("initialize") => json!({
                    "protocolVersion": agent_protocols::PROTOCOL_VERSION,
                    "serverInfo": {"name": "loopback"}
                }),
                Some("tools/list") => json!({"tools": [{
                    "name": "find",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"q": {"type": "string"}},
                        "required": ["q"]
                    }
                }]}),
                _ => json!({"content": [{"type": "text", "text": FOUND}]}),
            };
            let answer = match request.get("id") {
                Some(id) => {
                    serde_json::to_vec(&json!({"jsonrpc": "2.0", "id": id, "result": result}))
                        .unwrap()
                }
                None => Vec::new(),
            };
            let status = if answer.is_empty() { 202 } else { 200 };
            let stream = reader.get_mut();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                        answer.len()
                    )
                    .as_bytes(),
                )
                .unwrap();
            stream.write_all(&answer).unwrap();
        }
    });
    url
}

/// The catalogue episode: one search, answered with what the selected
/// supplier found.
pub(crate) fn episode() -> Episode {
    Episode {
        step: Step {
            tool: "web_search",
            args: json!({"query": "kiln temperature"}),
        },
        holds: |seen: &Observed<'_>| match seen.answer {
            ToolAnswer::Answered { result } => {
                let said = serde_json::to_string(result.as_map()).unwrap_or_default();
                if said.contains(FOUND) {
                    Ok(())
                } else {
                    Err(format!("the supplier's answer is not in {said}"))
                }
            }
            ToolAnswer::Failed { error } => Err(format!("failed: {:?}", error.as_map())),
        },
    }
}
