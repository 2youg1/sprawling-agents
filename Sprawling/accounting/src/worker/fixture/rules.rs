// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::Address;

pub(in crate::worker) fn ordinary_rules(rest: &str) -> String {
    format!("confidential = false\nwrite = \"everything\"\n{rest}")
}

pub(in crate::worker) fn shut_rules(rest: &str) -> String {
    format!("confidential = true\nwrite = \"everything\"\n{rest}")
}

pub(in crate::worker) fn lay_rules(city_root: &Path, building: &str, text: &str) {
    let addr = Address::parse(building).unwrap();
    let file = city::rules_path(city_root, &addr);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}
