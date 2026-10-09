// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One model's identity across providers (`crates/gateway/spec/Provider/Identity.lean`
//! §8-38).

use crate::endpoint::ModelFacts;

/// The identity two providers' rows share when they serve the same model:
/// the upstream's own canonical id where its list states one, else the
/// model id, either without the organisation prefix up to its last `/`,
/// in lowercase. Two rows are one model exactly when these strings are equal;
/// nothing here guesses at similar names, because one wrong match would
/// merge two models with different prices and windows into one row.
#[must_use]
pub fn canonical(facts: &ModelFacts) -> String {
    let named = facts.canonical.as_deref().unwrap_or(&facts.id);
    named
        .rsplit_once('/')
        .map_or(named, |(_, model)| model)
        .to_lowercase()
}
