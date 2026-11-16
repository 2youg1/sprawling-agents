// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxError, B3Hash, EventKind, EventRecord, RunId, Seq, TimeMs};
use serde::{Deserialize, Serialize};

pub const WIRE_V: u32 = 79;
mod ask;
mod monitor;

mod query;

pub use ask::{Answered, Ask, AskId, AskOutcome};
pub use monitor::{BEAT_MAX_MS, BEAT_MIN_MS, BeatMs, Monitoring, Sample, Watched};
pub use query::{QUERY_NAMES, Query};

use crate::answer::Answer;
use crate::command::{COMMAND_NAMES, WireCommand};

#[must_use]
pub fn schema_hash() -> B3Hash {
    let mut material = Vec::new();
    material.extend_from_slice(b"sprawling/wire/");
    material.extend_from_slice(&WIRE_V.to_le_bytes());
    for name in COMMAND_NAMES {
        material.push(b'C');
        material.extend_from_slice(name.as_bytes());
    }
    for name in QUERY_NAMES {
        material.push(b'Q');
        material.extend_from_slice(name.as_bytes());
    }
    for kind in EventKind::ALL {
        material.push(b'E');
        material.extend_from_slice(format!("{kind:?}").as_bytes());
    }
    B3Hash::digest(&material)
}

#[cfg(feature = "schema")]
#[must_use]
pub fn wire_schema() -> serde_json::Value {
    let mut generator = schemars::SchemaGenerator::default();
    generator.subschema_for::<ClientFrame>();
    generator.subschema_for::<ServerFrame>();
    let definitions = generator.take_definitions(true);
    serde_json::json!({
        "$schema": schemars::consts::meta_schemas::DRAFT2020_12,
        "title": "sprawling wire",
        "$defs": definitions,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Hello {
    pub wire_v: u32,
    pub schema: B3Hash,

    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Welcome {
    pub wire_v: u32,
    pub schema: B3Hash,

    pub resume_from: Option<Seq>,

    pub city: Option<Address>,

    pub epoch: Option<B3Hash>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ClientFrame {
    Hello(Hello),
    Command(Box<WireCommand>),
    Ask(Ask),
    Monitor(Monitoring),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ServerFrame {
    Welcome(Welcome),
    Event(Box<EventRecord>),
    Answered(Box<Answered>),
    Refusal(Box<AxError>),

    Delta(Delta),

    Log(LogLine),

    Lagged(Lagged),

    Output(LiveOutput),

    Monitor(Sample),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum OutputStream {
    Out,
    Err,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LiveOutput {
    pub run: RunId,
    pub stream: OutputStream,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Lagged {
    pub from: Seq,
    pub to: Seq,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Delta {
    pub run: RunId,
    pub increment: kernel::Increment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LogLevel {
    Refuse,
    Effect,
    Decide,
    Trace,
    Wire,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LogLine {
    pub seq: Seq,

    pub t: Option<TimeMs>,
    pub level: LogLevel,
    pub module: String,

    pub run: Option<RunId>,
    pub line: String,
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
#[path = "frames/tests.rs"]
mod tests;
