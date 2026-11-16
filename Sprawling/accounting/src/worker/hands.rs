// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::AxError;

use super::pool::Memory;

pub struct Hands {
    pub vault: gateway::Custodian,

    pub clock: Arc<dyn crate::Clock + Send + Sync>,

    pub monotonic: fn() -> std::time::Instant,

    pub machine: Box<dyn crate::Machine + Send>,

    pub read_memory: fn() -> Memory,

    pub read_volume: fn(&Path) -> Option<kernel::degradation::VolumeSpace>,

    pub reveal: fn(&Path, &kernel::Address) -> Result<(), AxError>,

    pub browsers: Browsers,

    pub desktop_program: DesktopProgram,

    pub recipe_for: fn(&str) -> Result<&'static crate::Recipe, AxError>,

    pub exec_host: ExecHost,

    pub seat_lane: SeatLane,

    pub shares: runtime::Shares,

    pub affinity: runtime::backlog::RunAffinity,

    pub budget: runtime::MachineBudget,
}

pub type SeatLane = fn() -> Box<dyn std::any::Any>;

pub type Browsers = fn(
    &Path,
    &storage::BlockOrigin,
    &city::BuildingRules,
) -> Result<Vec<Box<dyn kernel::Tool>>, AxError>;

pub type DesktopProgram = fn() -> std::io::Result<PathBuf>;

#[derive(Clone, Copy)]
pub struct ExecHost {
    pub python_wasm: fn() -> Option<PathBuf>,
    pub shell: fn() -> Option<PathBuf>,

    pub pwsh: fn() -> Option<PathBuf>,
    pub engine: fn() -> Result<Box<dyn runtime::Sandbox>, AxError>,

    pub confinement: fn(kernel::SandboxArm) -> Result<runtime::tools::Confined, AxError>,
    pub container: fn() -> Result<runtime::tools::ContainerRuntime, AxError>,
}
