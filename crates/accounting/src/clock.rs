// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The time the worker reads, handed to it rather than sampled by it
//! (accounting-SPEC.md 8-3).

use kernel::{AxError, TimeMs};

/// What time it is, for the worker and every lane it drives.
///
/// Every line the worker writes, every duration it measures and every
/// deadline it keeps is read here, so a scripted clock replays a whole
/// dispatch at the times the script chose.
///
/// A lane waiting out a provider's back-off sleeps until this clock
/// passes the deadline, so a scripted clock that never moves holds such
/// a run for as long as it stands still.
pub trait Clock {
    /// # Errors
    /// A clock that cannot be read, such as a wall clock set before the
    /// unix epoch.
    fn now(&self) -> Result<TimeMs, AxError>;
}
