// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{findings, title_findings};

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

/// A pull request title is judged as one subject line, because a
/// squash merge of two or more commits turns it into the subject main
/// receives; it has no body, so no ruling trailer is read.
#[test]
fn a_pull_request_title_is_judged_as_one_subject() {
    assert!(title_findings("card-S12.FIXPLAT03: judged before the merge").is_empty());
    let uncarded = [
        "card-S12.FIXPLAT01-02: the nightly platform checks, green again",
        "the nightly platform checks, green again",
    ];
    assert_eq!(uncarded.map(|title| title_findings(title).len()), [1, 1]);
}
