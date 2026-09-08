// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two identity files City Hall's residents are read from.
//!
//! They sit in the city's own reserved subtree rather than at the
//! residents' own addresses, and that placement is the whole design: no
//! write domain reaches the reserved subtree, so the Mayor cannot edit
//! who the Mayor is and the clerk cannot edit what it answers by. Every
//! other resident's `URBANITE.md` sits where that resident works, which
//! is right for a resident that a person dispatched and wrong for two
//! that serve the whole city.

use std::path::{Path, PathBuf};

use kernel::{Address, AxError};

use super::{storage, write_new};

/// Who the Mayor is: the identity file of `hall/mayor`.
pub const MAYOR_FILE: &str = "MAYOR.md";
/// Who the clerk is: the identity file of `hall/clerk`.
pub const CLERK_FILE: &str = "CLERK.md";

const MAYOR_TEMPLATE: &str = include_str!("../../../../docs/templates/MAYOR.md");
const CLERK_TEMPLATE: &str = include_str!("../../../../docs/templates/CLERK.md");

/// Where the identity file of a City Hall resident lives, and `None`
/// for every other address.
///
/// The two files sit in the city's own reserved subtree, which no write
/// domain reaches: the Mayor cannot edit who the Mayor is, and the clerk
/// cannot edit what it answers by. That is the one structural difference
/// between these two residents and every other one, whose `URBANITE.md`
/// sits at its own address.
#[must_use]
pub fn hall_identity_path(city_root: &Path, addr: &Address) -> Option<PathBuf> {
    let file = match addr.as_str() {
        kernel::consts_policy::HALL_MAYOR => MAYOR_FILE,
        kernel::consts_policy::HALL_CLERK => CLERK_FILE,
        _ => return None,
    };
    Some(city_root.join(kernel::RESERVED_PREFIX).join(file))
}

/// Lays down the two identity files a city raises City Hall with.
///
/// Nothing is overwritten: a person who has edited either file keeps
/// what they wrote, whatever is run against the city afterwards.
///
/// # Errors
/// Propagates a reserved subtree that cannot be created or written.
pub fn lay_out_hall_identities(city_root: &Path) -> Result<(), AxError> {
    let governed = city_root.join(kernel::RESERVED_PREFIX);
    std::fs::create_dir_all(&governed).map_err(|err| storage(&governed, &err))?;
    write_new(&governed.join(MAYOR_FILE), MAYOR_TEMPLATE)?;
    write_new(&governed.join(CLERK_FILE), CLERK_TEMPLATE)?;
    Ok(())
}
