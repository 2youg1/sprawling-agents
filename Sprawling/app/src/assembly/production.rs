// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::sync::Arc;

use kernel::{AxCode, AxError, TimeMs};

use crate::doctor::{PATIENCE, Platform, ThisMachine};
use accounting::worker::genesis::{Adopt, InitReport, form};
use accounting::worker::hands::{ExecHost, Hands};

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
            python: crate::doctor::host::usable_python,
            shell: crate::doctor::host::usable_shell,
            pwsh: crate::doctor::host::usable_pwsh,
            container: crate::doctor::host::container,
            confinement: crate::doctor::host::confinement,
        },
        seat_lane: crate::serving::placement::seat_lane,
        shares: crate::serving::placement::run_shares(),
        affinity: crate::serving::placement::run_affinity(),
        budget: crate::serving::placement::machine_budget(),
    }
}

pub fn init_city(city_root: &Path) -> Result<InitReport, AxError> {
    form_city(city_root, Adopt::Nothing)
}

pub fn form_city(city_root: &Path, adopt: Adopt<'_>) -> Result<InitReport, AxError> {
    let (vault, _notice) = crate::serving::open_vault();
    form(city_root, adopt, hands(vault))
}
