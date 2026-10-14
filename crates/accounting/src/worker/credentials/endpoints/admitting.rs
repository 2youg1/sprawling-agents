// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which model rows an attach registers (`crates/gateway/spec/Endpoint/Models.lean`
//! §8-10, D38 and D39).

/// What the person asked an attach to admit: the rows they ticked on
/// the endpoint's list, and the ids they typed.
pub(in crate::worker) struct Admission {
    pub(in crate::worker) ticked: Vec<String>,
    pub(in crate::worker) named: Vec<String>,
}

impl Admission {
    /// Nothing ticked and nothing typed: everything the endpoint serves.
    pub(in crate::worker) fn everything_served() -> Admission {
        Admission {
            ticked: Vec::new(),
            named: Vec::new(),
        }
    }

    /// Every id the person asked for, ticked first, each once.
    pub(super) fn asked(&self) -> Vec<String> {
        let mut asked: Vec<String> = Vec::new();
        for id in self.ticked.iter().chain(&self.named) {
            if !asked.contains(id) {
                asked.push(id.clone());
            }
        }
        asked
    }
}
