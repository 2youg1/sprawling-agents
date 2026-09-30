// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every hand the worker reaches this machine through, handed in once
//! when it is built (accounting-SPEC.md 8-11).
//!
//! Only what touches this machine is here. The judgement over it stays
//! with the worker: whether a run is handed a shell at all is the frozen
//! configuration's answer, and this module only says where one is.

use std::path::PathBuf;

use kernel::AxError;

/// What the exec tool takes from this machine: the python component and
/// the shell, each only where it is usable, and the engine this build
/// carries.
///
/// "Usable" is the doctor's judgement: a broken component or shell
/// arrives as `None`, because the exec tool could not tell it from a
/// working one.
#[derive(Clone, Copy)]
pub(crate) struct ExecHost {
    pub(crate) python_wasm: fn() -> Option<PathBuf>,
    pub(crate) shell: fn() -> Option<PathBuf>,
    pub(crate) engine: fn() -> Result<Box<dyn runtime::Sandbox>, AxError>,
}
