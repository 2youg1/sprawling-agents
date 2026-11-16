// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A request that nobody has verified has no verifier to name: the name
// exists only on `Pr<Verified>`, which only `Pr::verified` builds.

fn main() {
    let open = collab::Pr::open(
        collab::NodeId::parse("node-1").unwrap(),
        "lab/room1".to_owned(),
        "node-1".to_owned(),
    )
    .unwrap();
    let _ = open.verified_by();
}
