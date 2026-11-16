// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `crates/kernel/spec/PolicyLimit.lean` §8-73: a policy limit a caller minted cannot be spelled.
// The only values of these types are the constants in `consts_policy`,
// so the refusal a limit states is the only refusal there is — a
// caller cannot carry a looser limit to admit what the city refuses.

fn main() {
    let minted = kernel::policy_limit::ImagesPerTurn::new(9);
    let literal = kernel::policy_limit::ClockZonesMax(9);
}
