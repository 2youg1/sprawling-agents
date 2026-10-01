// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How far the person has got through this city's first-run guide
//! (accounting-SPEC.md 8-18-2, wire-SPEC.md 8-68).

use std::path::Path;

use kernel::AxError;
use wire::GuideProgress;

/// Not built yet: every city reads as a guide at its start.
pub(crate) fn read(_city_root: &Path) -> Result<GuideProgress, AxError> {
    Ok(GuideProgress::default())
}

/// Not built yet: the progress is not kept.
pub(crate) fn put(_city_root: &Path, _progress: &GuideProgress) -> Result<(), AxError> {
    Ok(())
}
