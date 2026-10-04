// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What only this machine can hand a worker: the wall clock, the hands
//! a worker built here reaches this machine through, and a city formed
//! with them (`crates/accounting/Spec.lean` §8-11).
//!
//! The clock is sampled *here only* (determinism rule 2): every callee
//! takes time as a parameter or reads the clock it was handed, and the
//! sample stays in this file so that the rule keeps naming one place.

use std::path::Path;
use std::sync::Arc;

use kernel::{AxCode, AxError, TimeMs};

use crate::doctor::{PATIENCE, Platform, ThisMachine};
use accounting::worker::genesis::{Adopt, InitReport, form};
use accounting::worker::hands::{ExecHost, Hands};

/// The wall clock: the single sanctioned sampling point (clippy.toml
/// disallowed-methods), and the production `accounting::Clock`
/// (`crates/accounting/Spec.lean` §8-3). Everything below it takes `TimeMs` as a
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
/// host half as the doctor judges it (`crates/accounting/Spec.lean` §8-11).
///
/// The one place the production value is made, so every production
/// worker reaches this machine the same way.
#[must_use]
pub fn hands(vault: gateway::Custodian) -> Hands {
    Hands {
        vault,
        clock: Arc::new(SystemClock),
        monotonic: crate::serving::standing::monotonic_now,
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
            pwsh: crate::doctor::host::usable_pwsh,
            engine: crate::doctor::host::execution_engine,
        },
        seat_lane: crate::serving::placement::seat_lane,
        shares: crate::serving::placement::run_shares(),
        affinity: crate::serving::placement::run_affinity(),
    }
}

/// `sprawling init <dir>`: forms a city with this machine's hands and
/// adopts nothing; what is already in the directory is left alone, and
/// [`form_city`] is the entry that puts it under rules.
///
/// # Errors
/// Whatever [`form_city`] reports.
pub fn init_city(city_root: &Path) -> Result<InitReport, AxError> {
    form_city(city_root, Adopt::Nothing)
}

/// Forms a city in a directory with this machine's hands: the vault this
/// process opens, the wall clock and the rest of [`hands`].
///
/// # Errors
/// Refuses a directory that already has history, and propagates whatever
/// the ledger, the store or the filesystem says.
pub fn form_city(city_root: &Path, adopt: Adopt) -> Result<InitReport, AxError> {
    let (vault, _notice) = crate::serving::open_vault();
    form(city_root, adopt, hands(vault))
}
