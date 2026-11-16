// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, ToolCall, ToolOutcome};
use serde_json::{Map, Value};

use crate::clock::ClockStamp;
use crate::offload::OffloadSite;
use crate::sieve::{CommandKey, FilterTable, SieveHistory};

use super::{PackContext, SieveRequest, package};

pub const EXEC_CAP_BYTES: u64 = 16_384;

pub struct SieveSite<'a> {
    pub offload: OffloadSite<'a>,
    pub table: &'a FilterTable,
    pub history: &'a mut SieveHistory,
}

fn text_field<'a>(map: &'a Map<String, Value>, name: &str) -> &'a str {
    map.get(name).and_then(Value::as_str).unwrap_or("")
}

pub fn package_exec(
    call: &ToolCall,
    outcome: ToolOutcome,
    site: SieveSite<'_>,
    stamp: Option<ClockStamp>,
) -> Result<ToolOutcome, AxError> {
    let mut result = outcome.result.as_map().clone();
    if !result.contains_key("stdout") && !result.contains_key("stderr") {
        return Ok(outcome);
    }
    let key = CommandKey::of(&crate::tools::parse_arm(call.args.as_map())?);
    let exit_code = super::exit_code_in(&result);
    let mut text = text_field(&result, "stdout").to_owned();
    let stderr = text_field(&result, "stderr");
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(stderr);
    }
    let packaged = package(
        text.as_bytes(),
        PackContext {
            cap_bytes: EXEC_CAP_BYTES,
            stamp,
            net_notice: false,
            steer: None,
            reminder: None,
            offload: Some(site.offload),
            sieve: Some(SieveRequest {
                key,
                exit_code,
                table: site.table,
                history: site.history,
            }),
            adviser: None,
        },
    )?;
    result.remove("stdout");
    result.remove("stderr");
    result.insert("content".to_owned(), Value::String(packaged.content));
    let accounts = packaged
        .events
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<Value>, _>>()
        .map_err(|err| {
            AxError::failure(AxCode::InvalidArgs, "encode sieve account", err.to_string())
                .with_recovery(
                    "report this against runtime::pipeline::exec: a sieve account holds \
                     names and counts, and JSON refuses neither",
                )
        })?;
    result.insert(super::EXEC_ACCOUNTS.to_owned(), Value::Array(accounts));
    Ok(ToolOutcome {
        result: kernel::Payload::new(result)?,
        attachments: Vec::new(),
    })
}
