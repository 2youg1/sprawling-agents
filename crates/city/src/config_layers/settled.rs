// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the ladder settled, and which rung said it.
//!
//! [`super::load`] spends the rungs on one resolved configuration; these
//! answer with the rung kept beside the value, because a page that was
//! told only the resolved setting cannot say whether it is looking at a
//! room's own entry or at something the city states for every room.

use std::path::Path;

use kernel::{Address, AxError, Effort, SecondThreshold};

use super::ConfigLayer;
use super::ladder::{Ladder, Layer};

/// How hard the model is asked to think at `addr`, and the rung that
/// said so.
///
/// The same climb `load` makes, answered with the rung kept rather
/// than spent. A page told only the resolved setting cannot say
/// whether it is looking at this room's own entry or at something the
/// city states for every room, so it would have to read all three
/// files and climb the ladder a second time — and two climbs of one
/// ladder are two answers to one question.
///
/// `None` is a ladder that states nothing, which is this city
/// deliberately leaving the setting to the provider rather than
/// filling in a level nobody chose.
///
/// # Errors
/// Refuses an address with no building, an unreadable file, and a file
/// that does not parse, exactly as [`super::load`] does.
pub fn settled_effort(
    city_root: &Path,
    addr: &Address,
) -> Result<Option<(Effort, Layer)>, AxError> {
    Ok(Ladder::read(city_root, addr)?
        .tagged(ConfigLayer::effort)
        .resolve()
        .copied())
}

/// Where the context reminder's second rung sits at this address, and
/// which layer stated it. Absent means no layer stated one and the
/// city's own default answers.
///
/// # Errors
/// Refuses an address with no building, an unreadable file, and a file
/// that does not parse, exactly as [`super::load`] does.
pub fn settled_second(
    city_root: &Path,
    addr: &Address,
) -> Result<Option<(SecondThreshold, Layer)>, AxError> {
    Ok(Ladder::read(city_root, addr)?
        .tagged(ConfigLayer::second_threshold)
        .resolve()
        .copied())
}
