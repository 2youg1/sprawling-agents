// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::findings;

/// A subject with its card and a ruling spelled the one way it may be.
#[test]
fn a_carded_subject_and_the_one_ruling_pass() {
    let messages = [
        "card-S5.7B: CI calls the recipes a person runs\n\nbody\n",
        "card-S4.02: the wire\n\nVerdict: user-approved\n",
        "card-S0.DEPS: Bump serde from 1.0.1 to 1.0.2\n",
    ];
    assert_eq!(messages.map(|message| findings(message).len()), [0, 0, 0]);
}

/// Each malformed subject and each misspelt ruling is one finding.
#[test]
fn an_uncarded_subject_and_a_misspelt_ruling_are_refused() {
    let messages = [
        "fix the thing\n",
        "card-S5.7b: lower-case index\n",
        "card-S5: no index\n",
        "card-S5.7B:no space\n",
        "Merge branch 'x'\n",
        "card-S5.7B: ok subject\n\nVerdict: approved\n",
        "card-S5.7B: ok subject\n\nVerdict: user-approved, twice\n",
        "",
    ];
    assert_eq!(
        messages.map(|message| findings(message).len()),
        [1, 1, 1, 1, 1, 1, 1, 1]
    );
}
