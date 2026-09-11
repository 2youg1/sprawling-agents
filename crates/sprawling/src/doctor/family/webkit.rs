// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Safari, through the driver macOS ships beside it (sprawling-SPEC.md
//! section 8-57).
//!
//! Data, with no branch in it. The member this family looks for is the
//! driver rather than the browser: `safaridriver` is the only door into
//! a Safari session, and a person who has Safari and has never run
//! `safaridriver --enable` has no session either way.

use super::{Confidence, Member};
use crate::doctor::PerPlatform;

/// One member, and only on one platform.
pub(super) const MEMBERS: &[Member] = &[Member {
    name: "safari",
    program: "safaridriver",
    homepage: "https://www.apple.com/safari/",
    confidence: Confidence::Experimental,
    places: PerPlatform {
        windows: &[],
        macos: &["/usr/bin/safaridriver"],
        linux: &[],
    },
    start_menu: "",
}];
