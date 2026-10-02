// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every hand the worker reaches this machine through, handed in once
//! when it is built (`crates/accounting/spec/Worker.lean` §8-11).
//!
//! Only what touches this machine is here. The judgement over it stays
//! with the worker: whether a run is handed a shell at all is the frozen
//! configuration's answer, and this module only says where one is. The
//! model factory and the MCP connections are not hands: the worker builds
//! its own from `gateway` and `agent_protocols` (accounting D18).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::AxError;

use super::pool::Memory;

/// What the worker takes from this machine, as one value.
///
/// The hands always arrive together and are used together, so they
/// travel as one value rather than as a constructor's nine parameters
/// (accounting D20). A caller that needs one hand of its own
/// writes `Hands { clock, ..hands }`, or calls the matching `with_*`
/// door after the worker is built.
pub struct Hands {
    /// Where this city's credentials are kept: the one the process
    /// opened, or an in-memory one.
    pub vault: gateway::Custodian,
    /// What time it is, for the worker and every lane it drives, from
    /// the first line it opens with (`crates/accounting/spec/Clock.lean` §8-3).
    pub clock: Arc<dyn crate::Clock + Send + Sync>,
    /// Reads the monotonic clock, which only moves forward: how long a
    /// dispatch's preparation or a probe took is a span, and a span read
    /// off `clock` grows or shrinks when somebody sets the wall clock
    /// (sprawling-SPEC.md 8-129-2).
    pub monotonic: fn() -> std::time::Instant,
    /// Looks at this machine and installs onto it (`crates/accounting/spec/Machine.lean` §8-4).
    pub machine: Box<dyn crate::Machine + Send>,
    /// Reads this machine's memory at the door new work enters by
    /// (sprawling-SPEC.md 8-46-3).
    pub read_memory: fn() -> Memory,
    /// Reads the city's volume at the same door (sprawling-SPEC.md 8-116).
    pub read_volume: fn(&Path) -> Option<kernel::degradation::VolumeSpace>,
    /// Hands one of this city's paths to the desktop's file manager
    /// (sprawling-SPEC.md 8-60).
    pub reveal: fn(&Path, &kernel::Address) -> Result<(), AxError>,
    /// Builds the browser tools a building's rules ask for
    /// (sprawling-SPEC.md 8-45-2).
    pub browsers: Browsers,
    /// Where the desktop server a building's rules ask for is started
    /// from (sprawling-SPEC.md 8-4d).
    pub desktop_program: DesktopProgram,
    /// How this build installs one named item on this platform: the
    /// requirement table stays with the doctor (sprawling-SPEC.md,
    /// `doctor_install`).
    pub recipe_for: fn(&str) -> Result<&'static crate::Recipe, AxError>,
    /// Where the exec tool's interpreter, shell and engine come from.
    pub exec_host: ExecHost,
}

/// The browser tools a building's rules ask for: its own browser, then
/// the person's when they declared one (sprawling-SPEC.md 8-45-2).
pub type Browsers = fn(
    &Path,
    &storage::BlockOrigin,
    &city::BuildingRules,
) -> Result<Vec<Box<dyn kernel::Tool>>, AxError>;

/// Where the desktop server this binary carries is started from: the
/// running executable in production (sprawling-SPEC.md 8-4d).
pub type DesktopProgram = fn() -> std::io::Result<PathBuf>;

/// What the exec tool takes from this machine: the python component and
/// the shell, each only where it is usable, and the engine this build
/// carries.
///
/// "Usable" is the doctor's judgement: a broken component or shell
/// arrives as `None`, because the exec tool could not tell it from a
/// working one.
#[derive(Clone, Copy)]
pub struct ExecHost {
    pub python_wasm: fn() -> Option<PathBuf>,
    pub shell: fn() -> Option<PathBuf>,
    pub engine: fn() -> Result<Box<dyn runtime::Sandbox>, AxError>,
}
