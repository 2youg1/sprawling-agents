// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The hands a worker under test is built with, the clocks they read, and
//! a city formed with them (accounting-SPEC.md 8-11, 12-19).

use std::path::{Path, PathBuf};

use kernel::{Address, AxError, TimeMs};

use crate::worker::Memory;
use crate::worker::genesis::{Adopt, InitReport, form};
use crate::worker::hands::{ExecHost, Hands};

/// The hands a worker under test is built with: none of them reaches
/// this machine (accounting-SPEC.md 8-11).
///
/// An in-memory vault, a clock that reads the wall, a machine with
/// nothing on it, roomy memory and volume, a file manager and a desktop
/// program that refuse, no browsers, a requirement table that carries
/// nothing, no interpreter and no shell, and no execution engine. A test
/// that needs a real hand puts that one in: `Hands { reveal, ..hands() }`
/// or a `with_*` door.
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
            engine: absent_engine,
        },
    }
}

/// The wall clock, read the way the production clock reads it, for the
/// tests that write lines at the present time.
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

/// A city clock that leaps a minute forward at every read, from the
/// wall's present: what a person setting the wall clock, or a laptop
/// waking, does to a span read off the city's clock.
pub(crate) struct LeapingClock(std::sync::atomic::AtomicU64);

impl LeapingClock {
    /// How far one read moves the clock: a span read off it as two
    /// reads apart is at least this long.
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

/// The monotonic clock, read the way the served city's one monotonic
/// sampling point reads it, for the tests that lap an opening.
#[allow(
    clippy::disallowed_methods,
    reason = "test code: the fixture reads the monotonic clock like the production point"
)]
pub(crate) fn monotonic() -> std::time::Instant {
    std::time::Instant::now()
}

/// A machine with nothing on it, which installs nothing.
struct NoMachine;

impl crate::Machine for NoMachine {
    fn report(&self) -> wire::DoctorAnswer {
        wire::DoctorAnswer {
            items: Vec::new(),
            tiers: Vec::new(),
            sandbox: wire::DoctorSandbox {
                arm: wire::DoctorSandboxArm::CopiedTree,
                coverage: Vec::new(),
            },
            custody: wire::DoctorCustody {
                store: wire::DoctorCustodyStore::SessionMemory,
                keeps: wire::DoctorCustodyLifetime::ThisProcess,
                refusal: None,
            },
            core: wire::DoctorCore::HeldBySetting,
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

/// A city formed with the test hands, adopting nothing: what the
/// worker's tests stand on (accounting-SPEC.md 12-19).
pub(crate) fn init_city(city_root: &Path) -> Result<InitReport, AxError> {
    form(city_root, Adopt::Nothing, hands())
}

/// Memory with far more room than any run asks for.
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

/// A volume with far more free space than any floor asks for.
pub(crate) fn roomy_volume(_city: &Path) -> Option<kernel::degradation::VolumeSpace> {
    Some(kernel::degradation::VolumeSpace {
        free_bytes: 1 << 50,
        total_bytes: 1 << 51,
    })
}
