// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{Address, AxError, TimeMs};

use crate::worker::Memory;
use crate::worker::genesis::{Adopt, InitReport, form};
use crate::worker::hands::{ExecHost, Hands};

pub(crate) fn hands() -> Hands {
    Hands {
        vault: gateway::Custodian::in_memory(),
        clock: std::sync::Arc::new(WallClock),
        monotonic,
        machine: Box::new(NoMachine),
        read_memory: roomy_memory,
        read_volume: roomy_volume,
        reveal: refuse_reveal,
        browsers: no_browsers,
        desktop_program: no_desktop,
        recipe_for: no_recipe,
        exec_host: ExecHost {
            python_wasm: absent_path,
            shell: absent_path,
            pwsh: absent_path,
            engine: absent_engine,
            confinement: |arm| {
                Ok(runtime::tools::Confined::with_arm(
                    match arm {
                        kernel::SandboxArm::CopiedTree => runtime::tools::Confinement::CopiedTree,
                        kernel::SandboxArm::Native => runtime::tools::Confinement::WindowsJobObject,
                        kernel::SandboxArm::None
                        | kernel::SandboxArm::Container
                        | kernel::SandboxArm::Python => runtime::tools::Confinement::CopiedTree,
                    },
                    Some(std::env::temp_dir()),
                ))
            },
            container: || {
                Err(kernel::AxError::failure(
                    kernel::AxCode::SandboxDenied,
                    "probe a container daemon",
                    "the fixture has no daemon",
                )
                .with_recovery("supply a scripted daemon fixture"))
            },
        },
        seat_lane: no_seat,
        shares: runtime::Shares::Unset,
        affinity: runtime::backlog::RunAffinity::Os,
        budget: runtime::MachineBudget::unmetered(),
    }
}

fn no_seat() -> Box<dyn std::any::Any> {
    Box::new(())
}

pub(crate) struct WallClock;

impl crate::Clock for WallClock {
    #[allow(
        clippy::disallowed_methods,
        reason = "test code: the fixture's clock reads the wall like the production one"
    )]
    fn now(&self) -> Result<TimeMs, AxError> {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        Ok(TimeMs::new(u64::try_from(elapsed.as_millis()).unwrap()))
    }
}

pub(crate) struct LeapingClock(std::sync::atomic::AtomicU64);

impl LeapingClock {
    pub(crate) const LEAP_MS: u64 = 60_000;

    pub(crate) fn from_now() -> LeapingClock {
        let now = crate::Clock::now(&WallClock).unwrap();
        LeapingClock(std::sync::atomic::AtomicU64::new(now.value()))
    }
}

impl crate::Clock for LeapingClock {
    fn now(&self) -> Result<TimeMs, AxError> {
        Ok(TimeMs::new(self.0.fetch_add(
            Self::LEAP_MS,
            std::sync::atomic::Ordering::Relaxed,
        )))
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "test code: the fixture reads the monotonic clock like the production point"
)]
pub(crate) fn monotonic() -> std::time::Instant {
    std::time::Instant::now()
}

struct NoMachine;

impl crate::Machine for NoMachine {
    fn report(&self, _city: &std::path::Path) -> wire::DoctorAnswer {
        wire::DoctorAnswer {
            items: Vec::new(),
            tiers: Vec::new(),
            sandbox: wire::DoctorSandbox {
                arm: wire::DoctorSandboxArm::CopiedTree,
                named: wire::SandboxArm::CopiedTree,
                coverage: Vec::new(),
            },
            custody: wire::DoctorCustody {
                store: wire::DoctorCustodyStore::SessionMemory,
                keeps: wire::DoctorCustodyLifetime::ThisProcess,
                refusal: None,
            },
            core: wire::DoctorCore::HeldBySetting,
            scanning: wire::DoctorScanning::DoesNotApply,
        }
    }

    fn install(&self, item: &str, _runnable: &crate::Runnable<'_>) -> Result<(), AxError> {
        Err(AxError::failure(
            kernel::AxCode::ToolUnavailable,
            "install a tool",
            format!("{item}: the machine under test installs nothing"),
        )
        .with_recovery("hand the worker a scripted machine with `with_machine`"))
    }
}

pub(crate) fn init_city(city_root: &Path) -> Result<InitReport, AxError> {
    form(city_root, Adopt::Nothing, hands())
}

fn roomy_memory() -> Memory {
    Memory {
        physical: 1 << 40,
        available: 1 << 39,
    }
}

fn refuse_reveal(_city: &Path, addr: &Address) -> Result<(), AxError> {
    Err(AxError::failure(
        kernel::AxCode::ToolUnavailable,
        "reveal a path",
        addr.as_str().to_owned(),
    )
    .with_recovery("hand the worker a file manager with `reveal_with`"))
}

fn no_browsers(
    _city: &Path,
    _origin: &storage::BlockOrigin,
    _rules: &city::BuildingRules,
) -> Result<Vec<Box<dyn kernel::Tool>>, AxError> {
    Ok(Vec::new())
}

fn no_desktop() -> std::io::Result<PathBuf> {
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "the hands under test start no desktop server",
    ))
}

fn no_recipe(item: &str) -> Result<&'static crate::Recipe, AxError> {
    Err(AxError::failure(
        kernel::AxCode::InvalidArgs,
        "install a tool",
        format!("{item}: the requirement table under test carries nothing"),
    )
    .with_recovery("hand the worker a table with `recipe_for_with`"))
}

fn absent_path() -> Option<PathBuf> {
    None
}

fn absent_engine() -> Result<Box<dyn runtime::Sandbox>, AxError> {
    Ok(Box::new(runtime::AbsentSandbox))
}

pub(crate) fn roomy_volume(_city: &Path) -> Option<kernel::degradation::VolumeSpace> {
    Some(kernel::degradation::VolumeSpace {
        free_bytes: 1 << 50,
        total_bytes: 1 << 51,
    })
}
