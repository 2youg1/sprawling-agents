// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `cargo xtask acp-catalog`: the ACP registry's index, read once from its
//! CDN and written as the catalog snapshot this build ships
//! (`crates/agent_protocols/Spec.lean` §8-19, D13).
//!
//! Only the fields the city reads are kept - `id`, `name`, `version`,
//! `license`, `distribution`, and of a binary target its `cmd`, `args` and
//! `env` - so authors' addresses, icons, links and download URLs never
//! enter the tree; the index's `Date` and `ETag` are written beside them.
//! A person runs it when a release refreshes the catalog; no gate does,
//! because a gate that reached the network would go red for the CDN's
//! reasons. `curl` fetches, as it is on every machine that builds this.

use std::path::Path;
use std::process::Command;

use serde_json::{Map, Value};

use crate::report::XtaskError;

/// Where the registry publishes its index.
const INDEX: &str = "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";

/// The snapshot `agent_protocols::Catalog::bundled` reads.
const SNAPSHOT: &str = "crates/agent_protocols/catalog/registry.json";

/// The fields of one agent the city reads.
const KEPT: [&str; 5] = ["distribution", "id", "license", "name", "version"];

/// The fields of one binary target the city reads: binaries are found on
/// the search path, not downloaded, so the archive and its checksum stay
/// out until the city installs them.
const KEPT_BINARY: [&str; 3] = ["args", "cmd", "env"];

#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
pub(crate) fn write(root: &Path) -> Result<String, XtaskError> {
    let fetched = Command::new("curl")
        .args(["--silent", "--show-error", "--fail", "--include", INDEX])
        .output()
        .map_err(|source| XtaskError::Io {
            path: format!("curl {INDEX}"),
            source,
        })?;
    if !fetched.status.success() {
        return Err(XtaskError::Cmd {
            cmd: format!("curl {INDEX}"),
            msg: String::from_utf8_lossy(&fetched.stderr).into_owned(),
        });
    }
    let text = String::from_utf8_lossy(&fetched.stdout).replace("\r\n", "\n");
    let (head, body) = text
        .split_once("\n\n")
        .ok_or_else(|| doc("the answer has no body"))?;
    let header = |name: &str| {
        head.lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.trim()
                    .eq_ignore_ascii_case(name)
                    .then(|| value.trim().to_owned())
            })
            .ok_or_else(|| doc(&format!("the answer has no {name} header")))
    };
    let index: Value = serde_json::from_str(body).map_err(|err| doc(&err.to_string()))?;
    let agents = index
        .get("agents")
        .and_then(Value::as_array)
        .ok_or_else(|| doc("the index has no `agents` list"))?;
    let kept: Vec<Value> = agents
        .iter()
        .filter_map(Value::as_object)
        .map(|agent| {
            let mut kept = picked(agent, &KEPT);
            if let Some(Value::Object(targets)) = kept
                .get_mut("distribution")
                .and_then(|distribution| distribution.get_mut("binary"))
            {
                for target in targets.values_mut() {
                    if let Value::Object(fields) = target {
                        *target = Value::Object(picked(fields, &KEPT_BINARY));
                    }
                }
            }
            Value::Object(kept)
        })
        .collect();
    let count = kept.len();
    let snapshot = Value::Object(Map::from_iter([
        ("agents".to_owned(), Value::Array(kept)),
        ("date".to_owned(), Value::String(header("date")?)),
        ("etag".to_owned(), Value::String(header("etag")?)),
        (
            "version".to_owned(),
            index.get("version").cloned().unwrap_or(Value::Null),
        ),
    ]));
    let mut written =
        serde_json::to_string_pretty(&snapshot).map_err(|err| doc(&err.to_string()))?;
    written.push('\n');
    let path = root.join(SNAPSHOT);
    std::fs::write(&path, written).map_err(|source| XtaskError::Io {
        path: SNAPSHOT.to_owned(),
        source,
    })?;
    Ok(format!("{SNAPSHOT}: {count} agents\n"))
}

fn picked(object: &Map<String, Value>, keys: &[&str]) -> Map<String, Value> {
    keys.iter()
        .filter_map(|key| Some(((*key).to_owned(), object.get(*key)?.clone())))
        .collect()
}

fn doc(msg: &str) -> XtaskError {
    XtaskError::Doc {
        file: INDEX.to_owned(),
        msg: msg.to_owned(),
    }
}
