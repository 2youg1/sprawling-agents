// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, reason = "test code")]

use super::Audience;

#[test]
fn a_file_is_read_by_an_agent() {
    let file = tempfile::tempfile().unwrap();
    assert_eq!(Audience::of(&file), Audience::Agent);
}
