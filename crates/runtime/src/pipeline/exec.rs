// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One `exec` result as the model reads it.
//!
//! The command's stdout and stderr become one text, go through
//! [`package`] under the command's key, and come back as `content`
//! with the sieve's account beside it. Every other field of the result
//! (the arm, the exit code, the names the child inherited, what the
//! backlog handed back) stays where the tool put it. This is the one
//! door for the shape: the city and citysim both come through it, so a
//! window the simulator replays is the window the product showed.

use kernel::{AxCode, AxError, ToolCall, ToolOutcome};
use serde_json::{Map, Value};

use crate::clock::ClockStamp;
use crate::offload::OffloadSite;
use crate::sieve::{CommandKey, FilterTable, SieveHistory};

use super::{PackContext, SieveRequest, package};

/// A command result's byte budget in the window. One value for both
/// callers; the sieve's floor and stages are the real shrink, this is
/// the ceiling behind them.
pub const EXEC_CAP_BYTES: u64 = 16_384;

/// Where a sieved result's original goes and what decides the cut:
/// the tee's site, the run's filter table, and the run's history.
pub struct SieveSite<'a> {
    pub offload: OffloadSite<'a>,
    pub table: &'a FilterTable,
    pub history: &'a mut SieveHistory,
}

fn text_field<'a>(map: &'a Map<String, Value>, name: &str) -> &'a str {
    map.get(name).and_then(Value::as_str).unwrap_or("")
}

/// Sieves and packages one `exec` outcome.
///
/// A result with neither `stdout` nor `stderr` - a command handed to
/// the background - is returned as it came: there is no text to cut
/// and pretending there was would put an empty `content` where the
/// handle is.
///
/// # Errors
/// Propagates a call whose arm does not parse, and whatever the
/// pipeline reports about the tee or the cut.
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
    let exit_code = result.get("exit_code").and_then(Value::as_i64);
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
        })?;
    result.insert("sieve".to_owned(), Value::Array(accounts));
    Ok(ToolOutcome {
        result: kernel::Payload::new(result)?,
    })
}
