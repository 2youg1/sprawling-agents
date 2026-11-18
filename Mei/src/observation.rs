// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The single writer for non-message `mei.obs` JSONL records.

use std::io::Write;

use serde_json::Value;

use crate::error::{Error, Result, protocol};

pub fn write(trace: &mut dyn Write, observation: &Value, action: &str) -> Result<()> {
    serde_json::to_writer(&mut *trace, observation).map_err(|err| {
        protocol(action, err.to_string(), "keep stderr writable").with_path("trace")
    })?;
    trace
        .write_all(b"\n")
        .and_then(|()| trace.flush())
        .map_err(|err| {
            Error::from(
                protocol(action, err.to_string(), "keep stderr writable").with_path("trace"),
            )
        })
}
