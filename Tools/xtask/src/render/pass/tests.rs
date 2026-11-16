// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

/// A run judges the three widths the columns are laid out at, and the
/// two conditions the page is painted under.
#[test]
fn a_run_with_no_width_named_opens_every_pass() {
    let passes = wanted().unwrap();
    let widths: Vec<u32> = passes.iter().map(|pass| pass.width).collect();
    assert_eq!(widths, vec![768, 1280, 2560, 1280, 1280]);
}

/// The pass names itself in the words a violation carries.
#[test]
fn a_pass_says_how_wide_and_how_lit_it_is() {
    let called: Vec<String> = wanted().unwrap().iter().map(|pass| pass.called()).collect();
    assert!(
        called
            .iter()
            .any(|name| name == "at 1280px, light in the colours it authored")
            && called
                .iter()
                .any(|name| name == "at 1280px, dark in forced colours"),
        "{}",
        called.join(" | ")
    );
}

/// A pass that asked for the light page and measured the dark one has
/// measured some other page, and says so instead of reporting green.
#[test]
fn a_light_pass_that_drew_a_dark_page_is_caught() {
    let passes = wanted().unwrap();
    let light = passes
        .iter()
        .find(|pass| pass.theme() == "light")
        .copied()
        .unwrap();
    assert!(light.disagrees(&Reported::read("0", "dark")).is_some());
    assert!(light.disagrees(&Reported::read("0", "light")).is_none());
}

/// And a forced-colour pass the engine did not put in forced colours.
/// Its lighting is the system's to decide, so it is not asked about.
#[test]
fn a_forced_colour_pass_the_engine_ignored_is_caught() {
    let passes = wanted().unwrap();
    let forced = passes
        .iter()
        .find(|pass| pass.draws_forced_colours())
        .copied()
        .unwrap();
    assert!(forced.disagrees(&Reported::read("0", "dark")).is_some());
    assert!(forced.disagrees(&Reported::read("1", "dark")).is_none());
    assert!(forced.disagrees(&Reported::read("1", "light")).is_none());
}
