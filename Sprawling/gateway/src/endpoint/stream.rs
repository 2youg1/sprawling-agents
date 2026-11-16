// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::BufRead as _;

use kernel::{AxError, ModelRequest, ModelReturn};
use serde_json::Value;

use crate::dialect;

use super::config::Endpoint;
use super::failure::{ProviderFailure, provider_err};

impl Endpoint {
    pub(crate) fn stream(
        &mut self,
        req: &ModelRequest,
        onto: kernel::Increments<'_>,
        early: kernel::EarlyCalls<'_>,
    ) -> Result<ModelReturn, AxError> {
        let mut wire = self.wire_request(req)?;
        if let Some(map) = wire.as_object_mut() {
            map.insert("stream".to_owned(), Value::Bool(true));

            if matches!(self.config.dialect, kernel::DialectKind::OpenAi) {
                map.insert(
                    "stream_options".to_owned(),
                    serde_json::json!({ "include_usage": true }),
                );
            }
        }
        let request = self.chat_post(req)?.header("accept", "text/event-stream");

        let mut sending = request
            .json(&wire)
            .build()
            .map_err(|err| provider_err("call provider", &ProviderFailure::Unbuilt(&err)))?;
        *sending.timeout_mut() = None;
        let response = self
            .client
            .execute(sending)
            .map_err(|err| provider_err("call provider", &ProviderFailure::Exchange(&err)))?;
        let status = response.status();
        if !status.is_success() {
            let headers = response.headers().clone();
            return Err(provider_err(
                "call provider",
                &ProviderFailure::refusal(
                    &self.config.base_url,
                    status,
                    &headers,
                    &response.text(),
                ),
            ));
        }

        let settled = if answers_whole(&response) {
            let mut body = String::new();
            self.lines_of(response, |line| {
                body.push_str(&line);
                body.push('\n');
                Ok(())
            })?;
            serde_json::from_str(&body).map_err(|err| {
                provider_err(
                    "read provider response",
                    &ProviderFailure::Unreadable(err.to_string()),
                )
            })?
        } else {
            let mut frames = dialect::StreamFrames::new(self.config.dialect);
            self.lines_of(response, |line| {
                if let Some(frame) = frame_of(&line) {
                    if let Some(held) = dialect::increment_of(self.config.dialect, &frame) {
                        onto(&held);
                    }
                    if let Some(call) = frames.retain_and_complete(frame)? {
                        early(&call);
                    }
                }
                Ok(())
            })?;

            frames.finish()?
        };
        self.returned(&settled)
    }

    fn lines_of(
        &self,
        response: reqwest::blocking::Response,
        mut each: impl FnMut(String) -> Result<(), AxError>,
    ) -> Result<(), AxError> {
        let quiet_ms = self
            .config
            .stream_idle_timeout_ms
            .unwrap_or(self.config.timeout_ms);
        let quiet = std::time::Duration::from_millis(quiet_ms);
        let (lines, arriving) = std::sync::mpsc::channel::<std::io::Result<String>>();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(response).lines() {
                let broke = line.is_err();
                if lines.send(line).is_err() || broke {
                    return;
                }
            }
        });
        loop {
            let line = match arriving.recv_timeout(quiet) {
                Ok(Ok(line)) => line,
                Ok(Err(err)) => {
                    return Err(provider_err(
                        "read provider response",
                        &ProviderFailure::Cut(&err),
                    ));
                }

                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    return Err(provider_err(
                        "read provider response",
                        &ProviderFailure::Silence { quiet_ms },
                    ));
                }
            };
            each(line)?;
        }
    }
}

fn answers_whole(response: &reqwest::blocking::Response) -> bool {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

fn frame_of(line: &str) -> Option<Value> {
    let payload = line.strip_prefix("data:")?.trim();

    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }

    serde_json::from_str::<Value>(payload).ok()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
