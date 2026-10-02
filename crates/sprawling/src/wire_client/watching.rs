// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling gauge --at`: watch a served city's monitor over the wire
//! and write each reading to stdout (`crates/sprawling/spec/WireClient.lean` §8-97).

use std::collections::VecDeque;
use std::io::Write;
use std::time::Duration;

use futures_util::SinkExt;
use kernel::{AxCode, AxError};
use sprawling::audience::Audience;
use sprawling::monitor::top::screen;
use sprawling::monitor::{CAPACITY, Sample};
use tokio_tungstenite::tungstenite::Message;

use super::{Unheard, hello, malformed, next_frame, unreachable_city};
use crate::gauge::lines::city_line;

/// Five empty beats: the city has stopped, or stopped sending.
const SILENCE: Duration = Duration::from_secs(5);

/// The columns a screen row spends before its curve: label and reading.
const BEFORE_CURVE: usize = 36;

/// The terminal width assumed when `COLUMNS` says nothing.
const COLUMNS: usize = 80;

/// Clears the terminal and puts the cursor top left.
const REDRAW: &str = "\u{1b}[H\u{1b}[2J";

/// Watches the city at `at` until it closes the socket or falls silent,
/// writing each reading in the form `audience` reads.
///
/// # Errors
/// `Unheard::NoCity` when nothing at `at` takes the connection or the
/// opening frames; `Unheard::Broken` when a frame breaks mid-read,
/// writing to stdout fails, or this process cannot start the runtime.
pub(crate) fn top(at: &str, token: Option<&str>, audience: Audience) -> Result<(), Unheard> {
    let greeting = serde_json::to_string(&hello(token))
        .map_err(|err| Unheard::Broken(malformed("encode the greeting", &err.to_string())))?;
    let watch = serde_json::to_string(&wire::ClientFrame::Monitor(wire::Monitoring::Watch))
        .map_err(|err| Unheard::Broken(malformed("encode the watch frame", &err.to_string())))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            Unheard::Broken(
                AxError::failure(
                    AxCode::StorageFatal,
                    "start the async runtime",
                    err.to_string(),
                )
                .with_recovery("close some programs and try again"),
            )
        })?
        .block_on(watch_until_silent(at, [greeting, watch], audience))
}

async fn watch_until_silent(
    at: &str,
    opening: [String; 2],
    audience: Audience,
) -> Result<(), Unheard> {
    let no_city = |why: String| Unheard::NoCity(unreachable_city(at, &why));
    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{at}/ws"))
        .await
        .map_err(|err| no_city(err.to_string()))?;
    for frame in opening {
        socket
            .send(Message::Text(frame.into()))
            .await
            .map_err(|err| no_city(err.to_string()))?;
    }
    let curve_width = std::env::var("COLUMNS")
        .ok()
        .and_then(|columns| columns.parse::<usize>().ok())
        .unwrap_or(COLUMNS)
        .saturating_sub(BEFORE_CURVE);
    let mut history = VecDeque::new();
    let mut stdout = std::io::stdout().lock();
    while let Some(text) = next_frame(&mut socket, SILENCE)
        .await
        .map_err(Unheard::Broken)?
    {
        if let Some(shown) = shown(&text, &mut history, audience, curve_width) {
            stdout
                .write_all(shown.as_bytes())
                .and_then(|()| stdout.flush())
                .map_err(|err| {
                    Unheard::Broken(
                        AxError::failure(
                            AxCode::StorageFatal,
                            "write a reading to stdout",
                            err.to_string(),
                        )
                        .with_recovery("stdout was closed; run `sprawling gauge` again"),
                    )
                })?;
        }
    }
    Ok(())
}

/// The text one received frame puts on stdout, keeping the reading in
/// `history`; `None` for a frame that is not a monitor reading.
pub(crate) fn shown(
    text: &str,
    history: &mut VecDeque<Sample>,
    audience: Audience,
    curve_width: usize,
) -> Option<String> {
    let Ok(wire::ServerFrame::Monitor(sample)) = serde_json::from_str(text) else {
        return None;
    };
    if history.len() >= CAPACITY {
        history.pop_front();
    }
    history.push_back(sample);
    match audience {
        Audience::Agent => city_line(&sample).ok().map(|line| format!("{line}\n")),
        Audience::Person => Some(format!(
            "{REDRAW}{}\n",
            screen(history.make_contiguous(), curve_width)
        )),
    }
}

#[cfg(test)]
#[path = "watching/tests.rs"]
mod tests;
