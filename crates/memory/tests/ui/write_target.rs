// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A write target is cleared at one constructor and its field is
// private: a path that skipped the alias check cannot be spelled, so no
// write face can be handed one.

fn main() {
    let forged = storage::WriteTarget(std::path::PathBuf::from("lab/x"));
    let _ = forged.as_path();
}
