// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[cfg(windows)]
pub mod confinement;
pub mod cpu_set;
pub mod step;

#[cfg(windows)]
pub mod boundary;
#[cfg(windows)]
pub mod capture;
#[cfg(windows)]
pub mod clipboard;
#[cfg(windows)]
pub mod cpu;
#[cfg(windows)]
pub mod dpi;
#[cfg(windows)]
pub mod ended;
#[cfg(all(windows, feature = "fixture"))]
pub mod fixture;
#[cfg(windows)]
mod leaf;
#[cfg(all(windows, test))]
mod reference;
#[cfg(windows)]
pub mod top_level;
