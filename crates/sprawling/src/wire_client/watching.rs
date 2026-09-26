// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling top`: watch a served city's monitor over the wire and
//! write each reading to stdout (sprawling-SPEC.md 8-93).

use std::collections::VecDeque;
use std::io::Write;
use std::time::Duration;

use futures_util::SinkExt;
use kernel::{AxCode, AxError};
use sprawling::monitor::top::{json_line, screen};
use sprawling::monitor::{CAPACITY, Sample};
use tokio_tungstenite::tungstenite::Message;

use super::{hello, malformed, next_frame, unreachable_city};

/// Five empty beats: the city has stopped, or stopped sending.
const SILENCE: Duration = Duration::from_secs(5);

/// The columns a screen row spends before its curve: label and reading.
const BEFORE_CURVE: usize = 36;

/// The terminal width assumed when `COLUMNS` says nothing.
const COLUMNS: usize = 80;

/// Clears the terminal and puts the cursor top left.
const REDRAW: &str = "\u{1b}[H\u{1b}[2J";

/// How a reading is written: redrawn on a terminal, one JSON line a
/// second for anything else, an agent included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Output {
    Screen,
    Lines,
}

/// Watches the city at `at` until it closes the socket or falls silent.
///
/// # Errors
/// The city cannot be reached, refuses the greeting, or a frame breaks
/// mid-read; writing to stdout fails.
pub(crate) fn top(at: &str, token: Option<&str>, output: Output) -> Result<(), AxError> {
    let greeting = serde_json::to_string(&hello(token))
        .map_err(|err| malformed("encode the greeting", &err.to_string()))?;
    let watch = serde_json::to_string(&channels::ClientFrame::Monitor(channels::Monitoring::Watch))
        .map_err(|err| malformed("encode the watch frame", &err.to_string()))?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the async runtime",
                err.to_string(),
            )
            .with_recovery("close some programs and try again")
        })?
        .block_on(watch_until_silent(at, [greeting, watch], output))
}

async fn watch_until_silent(at: &str, opening: [String; 2], output: Output) -> Result<(), AxError> {
    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{at}/ws"))
        .await
        .map_err(|err| unreachable_city(at, &err.to_string()))?;
    for frame in opening {
        socket
            .send(Message::Text(frame.into()))
            .await
            .map_err(|err| unreachable_city(at, &err.to_string()))?;
    }
    let curve_width = std::env::var("COLUMNS")
        .ok()
        .and_then(|columns| columns.parse::<usize>().ok())
        .unwrap_or(COLUMNS)
        .saturating_sub(BEFORE_CURVE);
    let mut history = VecDeque::new();
    let mut stdout = std::io::stdout().lock();
    while let Some(text) = next_frame(&mut socket, SILENCE).await? {
        if let Some(shown) = shown(&text, &mut history, output, curve_width) {
            stdout
                .write_all(shown.as_bytes())
                .and_then(|()| stdout.flush())
                .map_err(|err| {
                    AxError::failure(
                        AxCode::StorageFatal,
                        "write a reading to stdout",
                        err.to_string(),
                    )
                    .with_recovery("stdout was closed; run `sprawling top` again")
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
    output: Output,
    curve_width: usize,
) -> Option<String> {
    let Ok(channels::ServerFrame::Monitor(sample)) = serde_json::from_str(text) else {
        return None;
    };
    if history.len() >= CAPACITY {
        history.pop_front();
    }
    history.push_back(sample);
    match output {
        Output::Lines => json_line(&sample).ok().map(|line| format!("{line}\n")),
        Output::Screen => Some(format!(
            "{REDRAW}{}\n",
            screen(history.make_contiguous(), curve_width)
        )),
    }
}

#[cfg(test)]
#[path = "watching/tests.rs"]
mod tests;
