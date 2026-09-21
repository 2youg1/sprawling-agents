// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a building's rules look like in a test, and where they go.
//!
//! Both, in one place. Twenty-five tests used to spell a rules file
//! themselves, so the format had a home in each of them and a change to
//! it had to find every one — which is the same defect the format
//! change was made to remove from the product.

use std::path::Path;

use kernel::Address;

/// The rules an ordinary test building is laid out with: the two
/// answers every file has to give, then whatever the test is about.
pub(in crate::assembly) fn ordinary_rules(rest: &str) -> String {
    format!("confidential = false\nwrite = \"everything\"\n{rest}")
}

/// The same, for a building whose data does not leave.
pub(in crate::assembly) fn shut_rules(rest: &str) -> String {
    format!("confidential = true\nwrite = \"everything\"\n{rest}")
}

/// Lays a building's rules where the city reads them.
///
/// Through `city::rules_path` rather than by joining a file name: a
/// fixture that spells the path itself is a second authority for where
/// the rules live, and it goes on passing after the real one has moved.
pub(in crate::assembly) fn lay_rules(city_root: &Path, building: &str, text: &str) {
    let addr = Address::parse(building).unwrap();
    let file = city::rules_path(city_root, &addr);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}
