// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;
use std::io::Write;

use serde_json::{Map, Value};

use crate::error::{Result, protocol};

#[derive(Debug)]
pub struct History {
    bytes: Vec<u8>,
    count: usize,
    last_start: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            bytes: br#"{"messages":["#.to_vec(),
            count: 0,
            last_start: 0,
        }
    }
}

impl History {
    pub(crate) fn push(&mut self, message: &Value) -> Result<usize> {
        let previous = self.bytes.len();
        if self.count > 0 {
            self.bytes.push(b',');
        }
        let start = self.bytes.len();
        if let Err(err) = serde_json::to_writer(&mut self.bytes, message) {
            self.bytes.truncate(previous);
            return Err(encoding(err));
        }
        self.last_start = start;
        let index = self.count;
        self.count = self.count.saturating_add(1);
        Ok(index)
    }

    fn trace_last(&self, trace: &mut dyn Write, stop: Option<&str>) -> Result<()> {
        let (_, message) = self.bytes.split_at(self.last_start);
        let mut line = || -> std::io::Result<()> {
            if let Some(stop) = stop {
                let fields = message
                    .strip_suffix(b"}")
                    .ok_or_else(|| std::io::Error::other("expected an encoded message object"))?;
                trace.write_all(fields)?;
                trace.write_all(br#","stop_reason":"#)?;
                serde_json::to_writer(&mut *trace, stop)?;
                trace.write_all(b"}")?;
            } else {
                trace.write_all(message)?;
            }
            trace.write_all(b"\n")?;
            trace.flush()
        };
        line().map_err(|err| {
            protocol(
                "trace an appended message",
                err.to_string(),
                "keep stderr writable",
            )
            .with_path("trace")
            .into()
        })
    }
}

fn encoding(err: serde_json::Error) -> crate::Error {
    protocol(
        "serialize a Messages request",
        err.to_string(),
        "use JSON-compatible values in the request",
    )
    .into()
}

pub struct Request {
    pub(crate) history: History,
    settings: Vec<u8>,
}

impl Request {
    pub fn new(mut fields: Map<String, Value>, history: History) -> Result<Self> {
        fields.remove("messages");
        let mut settings = vec![b']'];
        for (key, value) in fields {
            settings.push(b',');
            serde_json::to_writer(&mut settings, &key).map_err(encoding)?;
            settings.push(b':');
            serde_json::to_writer(&mut settings, &value).map_err(encoding)?;
        }
        settings.push(b'}');
        Ok(Self { history, settings })
    }

    pub(crate) fn parts(&self) -> [&[u8]; 2] {
        [&self.history.bytes, &self.settings]
    }
}

pub(crate) trait Transcript {
    fn post(&self, client: &crate::transport::Client, trace: &mut dyn Write) -> Result<Value>;
    fn trace_prompt(&self, trace: &mut dyn Write) -> Result<()>;
    fn store(
        &mut self,
        fields: Map<String, Value>,
        stop: Option<&str>,
        trace: &mut dyn Write,
    ) -> Result<(usize, Cow<'_, Value>)>;
}

impl Transcript for Request {
    fn post(&self, client: &crate::transport::Client, trace: &mut dyn Write) -> Result<Value> {
        client.post_parts(self.parts(), trace)
    }

    fn trace_prompt(&self, trace: &mut dyn Write) -> Result<()> {
        self.history.trace_last(trace, None)
    }

    fn store(
        &mut self,
        fields: Map<String, Value>,
        stop: Option<&str>,
        trace: &mut dyn Write,
    ) -> Result<(usize, Cow<'_, Value>)> {
        let message = Value::Object(fields);
        let index = self.history.push(&message)?;
        self.history.trace_last(trace, stop)?;
        Ok((index, Cow::Owned(message)))
    }
}

impl Transcript for Value {
    fn post(&self, client: &crate::transport::Client, trace: &mut dyn Write) -> Result<Value> {
        client.post(self, trace)
    }

    fn trace_prompt(&self, trace: &mut dyn Write) -> Result<()> {
        let Some(prompt) = self
            .get("messages")
            .and_then(Value::as_array)
            .and_then(|messages| messages.last())
            .filter(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        else {
            return Ok(());
        };
        crate::observation::write(trace, prompt, "trace the initial user prompt")
    }

    fn store(
        &mut self,
        fields: Map<String, Value>,
        stop: Option<&str>,
        trace: &mut dyn Write,
    ) -> Result<(usize, Cow<'_, Value>)> {
        let messages = match self.get_mut("messages") {
            Some(Value::Array(messages)) => messages,
            value => {
                let found = value.map_or_else(|| "<missing>".to_owned(), |value| value.to_string());
                return Err(protocol(
                    "append a message",
                    "request.messages is missing or is not an array",
                    "construct one Messages request with messages as an array before calling agent::run",
                ).with_path("request.messages").with_value(found).into());
            }
        };
        let index = messages.len();
        let mut fields = fields;
        if let Some(stop) = stop {
            fields.insert("stop_reason".to_owned(), Value::from(stop));
        }
        let mut message = Value::Object(fields);
        let logged = crate::observation::write(trace, &message, "trace an appended message");
        if let Some(fields) = message.as_object_mut() {
            fields.remove("stop_reason");
        }
        let stored = messages.push_mut(message);
        logged?;
        Ok((index, Cow::Borrowed(stored)))
    }
}
