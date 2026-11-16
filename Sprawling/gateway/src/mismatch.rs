// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, CacheCount, Payload, Tokens};
use serde_json::Value;

pub(crate) fn mismatch(path: &str, detail: &str) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "translate wire",
        format!("{path}: {detail}"),
    )
    .with_recovery(
        "the provider answered in a shape this dialect does not know; check that the \
         endpoint's dialect matches the provider and that its base url is the one its \
         documentation prints",
    )
}

const FOUND_MAX: usize = 40;

pub(crate) fn mismatch_found(path: &str, detail: &str, found: &str) -> AxError {
    mismatch(path, &format!("{detail}: {}", quoted(found)))
}

fn quoted(found: &str) -> String {
    let head: String = found.chars().take(FOUND_MAX).collect();
    if head.chars().count() < found.chars().count() {
        format!("{head:?}\u{2026}")
    } else {
        format!("{head:?}")
    }
}

pub(crate) fn settled_tool_arguments(tool: &str, at: u64, raw: &str) -> Result<Value, AxError> {
    serde_json::from_str(raw).map_err(|err| {
        stream_cut(&format!(
            "tool `{tool}` at index {at} sent {} characters of arguments that stop before the \
             value ends ({err})",
            raw.chars().count()
        ))
    })
}

pub(crate) fn stream_cut(detail: &str) -> AxError {
    AxError::failure(
        AxCode::Provider,
        "read a streamed answer",
        detail.to_owned(),
    )
    .effect_unknown()
    .with_recovery(
        "the reply stopped before its end, so the provider may have run and billed it; \
         the watchdog backs off and sends it again, until the run's retry limit or a Halt",
    )
}

pub(crate) fn require<'v>(wire: &'v Value, path: &str, key: &str) -> Result<&'v Value, AxError> {
    wire.get(key)
        .ok_or_else(|| mismatch(&format!("{path}.{key}"), "missing"))
}

pub(crate) fn as_str<'v>(value: &'v Value, path: &str) -> Result<&'v str, AxError> {
    value
        .as_str()
        .ok_or_else(|| mismatch(path, "expected string"))
}

fn as_u64(value: &Value, path: &str) -> Result<u64, AxError> {
    value
        .as_u64()
        .ok_or_else(|| mismatch(path, "expected unsigned integer"))
}

pub(crate) fn tokens_or_zero(usage: &Value, key: &str, path: &str) -> Result<Tokens, AxError> {
    Ok(cache_count(Some(usage), key, path)?.or_zero())
}

pub(crate) fn cache_count(
    usage: Option<&Value>,
    key: &str,
    path: &str,
) -> Result<CacheCount, AxError> {
    match usage.and_then(|held| held.get(key)) {
        None | Some(Value::Null) => Ok(CacheCount::Unreported),
        Some(value) => Ok(CacheCount::Reported(Tokens::new(as_u64(
            value,
            &format!("{path}.{key}"),
        )?))),
    }
}

pub(crate) fn payload_from(value: &Value, path: &str) -> Result<Payload, AxError> {
    serde_json::from_value(value.clone()).map_err(|err| mismatch(path, &err.to_string()))
}
