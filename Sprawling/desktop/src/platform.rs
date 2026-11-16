// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[cfg(not(windows))]
mod elsewhere;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
pub(crate) use elsewhere::Desk;
#[cfg(windows)]
pub(crate) use windows::Desk;
