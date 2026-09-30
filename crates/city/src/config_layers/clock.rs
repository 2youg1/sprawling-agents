// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[clock]` table: how often a layer's results carry the clock
//! line.
//!
//! The granularity and its spellings are `kernel`'s answer
//! (`kernel::ClockStampGranularity`), and what a granularity does to a
//! result is `runtime::clock`'s. What this module owns is the one key a
//! layer may write here. Time zones are not one of them: a stamp is
//! written in UTC, so a `zones` key would be a setting nothing reads,
//! and it is refused where the file is parsed (city-SPEC 12.7).

use kernel::ClockStampGranularity;
use serde::Deserialize;

/// What one layer's `[clock]` table states.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClockSection {
    pub(super) stamp: ClockStampGranularity,
}
