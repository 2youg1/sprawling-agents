// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The long-turn instrument (citysim-SPEC 8-9, 3-10):
//! `long_turn <steps> [every]`.
//!
//! After every `every` reads it prints `step <k> pid <pid>` and waits for
//! one line on its input, so that `just mem long-turn` can read this
//! process's counters through `cargo xtask mem` while the turn stands
//! still; the counters have that one reader. When the turn ends it
//! prints `done <completion> steps <n> window <bytes> ledger <bytes>`:
//! the last request window, and what the simulator's own ledger holds,
//! which every memory reading of this process includes.

use std::io::{BufRead, Write};
use std::process::ExitCode;

use citysim::{Pauses, long_turn};
use kernel::{AxCode, AxError};

/// How many reads pass between two pauses when the caller names none.
const EVERY: u32 = 100;

fn main() -> ExitCode {
    match run() {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, AxError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (steps, every) = match args.as_slice() {
        [steps] => (count(steps)?, EVERY),
        [steps, every] => (count(steps)?, count(every)?),
        _ => return Err(usage(&args.join(" "))),
    };
    let reading = long_turn(
        steps,
        Pauses::Every {
            steps: every,
            at: Box::new(pause),
        },
    )?;
    let window = reading.windows.last().copied().unwrap_or(0);
    Ok(format!(
        "done {} steps {steps} window {window} ledger {}",
        reading.completion, reading.ledger_bytes
    ))
}

/// Says where the turn stands and waits until the reader has read it.
fn pause(read: u32) -> Result<(), AxError> {
    let stalled = |err: std::io::Error| {
        AxError::failure(AxCode::InvalidArgs, "pause a long turn", err.to_string())
            .with_recovery("run the instrument through `just mem long-turn`, which answers it")
    };
    let mut out = std::io::stdout().lock();
    writeln!(out, "step {read} pid {}", std::process::id()).map_err(stalled)?;
    out.flush().map_err(stalled)?;
    let mut answer = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut answer)
        .map_err(stalled)?;
    Ok(())
}

fn count(raw: &str) -> Result<u32, AxError> {
    raw.parse::<u32>().map_err(|_| usage(raw))
}

fn usage(given: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "run a long turn",
        format!("`{given}` is not `<steps> [every]`"),
    )
    .with_recovery("give the number of steps, and optionally how many reads pass between pauses")
}
