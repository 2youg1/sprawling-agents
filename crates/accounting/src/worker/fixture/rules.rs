// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a building's rules look like in a test, and where they go.
//!
//! Both, in one place, so the format has one home in the tests as it
//! has one in the product, and a change to it does not have to find
//! every test that spells a rules file itself.

use std::path::Path;

use kernel::Address;

/// The rules an ordinary test building is laid out with: the two
/// answers every file has to give, then whatever the test is about.
pub(in crate::worker) fn ordinary_rules(rest: &str) -> String {
    format!("confidential = false\nwrite = \"everything\"\n{rest}")
}

/// The same, for a building whose data does not leave.
pub(in crate::worker) fn shut_rules(rest: &str) -> String {
    format!("confidential = true\nwrite = \"everything\"\n{rest}")
}

/// Lays a building's rules where the city reads them.
///
/// Through `city::rules_path` rather than by joining a file name: a
/// fixture that spells the path itself is a second authority for where
/// the rules live, and it goes on passing after the real one has moved.
pub(in crate::worker) fn lay_rules(city_root: &Path, building: &str, text: &str) {
    let addr = Address::parse(building).unwrap();
    let file = city::rules_path(city_root, &addr);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}
