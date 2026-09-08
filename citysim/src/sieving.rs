// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The sieve's world in a scenario: a CAS and an environment directory
//! for the tee, one filter table, and the run's history. An `exec`
//! result is put through `runtime::pipeline::package` with its command
//! key, the same door the city will use, and what the model sees is
//! recorded together with the sieve's own account.

use std::path::PathBuf;

use kernel::{AxCode, AxError, Payload, ToolCall, ToolOutcome};
use memory::Cas;
use runtime::clock::ClockStamp;
use runtime::offload::OffloadSite;
use runtime::{CommandKey, FilterTable, PackContext, SieveHistory, SieveRequest, package};
use serde_json::{Map, Value};

pub struct SieveWorld {
    pub cas: Cas,
    pub environment: PathBuf,
    pub table: FilterTable,
    history: SieveHistory,
}

impl SieveWorld {
    /// Opens the CAS under `root` and materializes rest files beside it.
    /// Opening the same root twice is the same world: the CAS is
    /// content-addressed and a rest file is written once.
    pub fn open(root: &std::path::Path, table: FilterTable) -> Result<SieveWorld, AxError> {
        let cas = Cas::open(&root.join("cas")).map_err(memory::MemoryError::into_ax)?;
        let environment = root.join("env");
        std::fs::create_dir_all(&environment).map_err(|err| {
            AxError::failure(AxCode::StorageFatal, "open sieve world", err.to_string())
        })?;
        Ok(SieveWorld {
            cas,
            environment,
            table,
            history: SieveHistory::default(),
        })
    }
}

fn text_field<'a>(map: &'a Map<String, Value>, name: &str) -> &'a str {
    map.get(name).and_then(Value::as_str).unwrap_or("")
}

/// One `exec` outcome, sieved and packaged. The result the model reads
/// is `{content, exit_code, sieve}`: the window text, the code, and the
/// `result_offloaded` payloads the pipeline produced.
pub(crate) fn package_exec(
    call: &ToolCall,
    outcome: &ToolOutcome,
    world: &mut SieveWorld,
    stamp: Option<ClockStamp>,
) -> Result<ToolOutcome, AxError> {
    let key = CommandKey::of(&runtime::parse_arm(call.args.as_map())?);
    let result = outcome.result.as_map();
    let exit_code = result.get("exit_code").and_then(Value::as_i64);
    let mut text = text_field(result, "stdout").to_owned();
    let stderr = text_field(result, "stderr");
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(stderr);
    }
    let packaged = package(
        text.as_bytes(),
        PackContext {
            cap_bytes: 16_384,
            stamp,
            net_notice: false,
            steer: None,
            offload: Some(OffloadSite {
                cas: &mut world.cas,
                environment: &world.environment,
            }),
            sieve: Some(SieveRequest {
                key,
                exit_code,
                table: &world.table,
                history: &mut world.history,
            }),
        },
    )?;
    let mut wrapped = Map::new();
    wrapped.insert("content".to_owned(), Value::String(packaged.content));
    if let Some(code) = exit_code {
        wrapped.insert("exit_code".to_owned(), Value::Number(code.into()));
    }
    let accounts = packaged
        .events
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()
        .map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode sieve account", err.to_string())
        })?;
    wrapped.insert("sieve".to_owned(), Value::Array(accounts));
    Ok(ToolOutcome {
        result: Payload::new(wrapped)?,
    })
}
