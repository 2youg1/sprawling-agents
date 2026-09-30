// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What only this machine can hand a worker: the wall clock, and the
//! hands a worker built here reaches this machine through
//! (accounting-SPEC.md 8-11).
//!
//! The clock is sampled *here only* (determinism rule 2): every callee
//! takes time as a parameter or reads the clock it was handed, and the
//! sample stays in this file so that the rule keeps naming one place.

use std::sync::Arc;

use kernel::{AxCode, AxError, TimeMs};

use super::hands::{ExecHost, Hands};
use crate::doctor::{PATIENCE, Platform, ThisMachine};

/// The wall clock: the single sanctioned sampling point (clippy.toml
/// disallowed-methods), and the production `accounting::Clock`
/// (accounting-SPEC.md 8-3). Everything below it takes `TimeMs` as a
/// parameter or reads the clock it was handed; what samples it outside
/// this module is handed it at construction, as the process log is.
pub struct SystemClock;

impl accounting::Clock for SystemClock {
    fn now(&self) -> Result<TimeMs, AxError> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the one sampling point: Main injects time"
        )]
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| {
                AxError::failure(AxCode::ConfigInvalid, "sample wall clock", err.to_string())
                    .with_recovery("fix the system clock; it reads before the unix epoch")
            })?;
        let millis = u64::try_from(elapsed.as_millis()).map_err(|_| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "sample wall clock",
                "beyond u64 millis",
            )
            .with_recovery(
                "set this machine's clock to the present day; it reads more than half a \
                 billion years after the unix epoch",
            )
        })?;
        Ok(TimeMs::new(millis))
    }
}

/// The hands a worker on this machine is built with: `vault`, the wall
/// clock, the doctor, the monitor's two counters, the file manager, the
/// browsers, this executable, the requirement table, and the exec tool's
/// host half as the doctor judges it (accounting-SPEC.md 8-11).
///
/// The one place the production value is made, so every production
/// worker reaches this machine the same way.
#[must_use]
pub fn hands(vault: gateway::Custodian) -> Hands {
    Hands {
        vault,
        clock: Arc::new(SystemClock),
        machine: Box::new(ThisMachine::new(Platform::current(), PATIENCE)),
        read_memory: crate::monitor::memory::read,
        read_volume: crate::monitor::volume::read,
        reveal: crate::revealing::reveal,
        browsers: crate::browser_tool::for_rules,
        desktop_program: std::env::current_exe,
        recipe_for: crate::doctor::recipe_for,
        exec_host: ExecHost {
            python_wasm: crate::doctor::host::usable_python_wasm,
            shell: crate::doctor::host::usable_shell,
            engine: crate::doctor::host::execution_engine,
        },
    }
}
