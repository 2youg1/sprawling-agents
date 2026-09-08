// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The sentence that names the way into one building's own pages.

use crate::lang::{Msg, fill, say};

/// The link into one building's own pages, said.
pub(super) fn read_what(lang: crate::lang::Lang, id: &str) -> String {
    fill(say(lang, Msg::CityReadWhat), &[("id", id)])
}
