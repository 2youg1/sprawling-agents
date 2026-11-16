// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

fn main() {
    let root = kernel::Delegator::root();
    let delegate = root.delegate(kernel::DelegateKind::Resident);
    let _ = kernel::Pursuit::declare(&delegate, "work all night".to_owned());
}
