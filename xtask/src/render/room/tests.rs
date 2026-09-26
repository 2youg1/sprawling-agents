// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

use browser::survey::{Cut, Overflow, Sampled, TextRun};

const AT: &str = "gallery 390 dark";

/// One box as the probe writes it: its tag and role, its name, where it
/// was drawn (`[left, top, width, height]`), and its nearest measured
/// ancestor.
fn el(tag_role: (&str, &str), name: &str, at: [i64; 4], parent: i64) -> Drawn {
    let [left, top, width, height] = at;
    Drawn {
        tag: tag_role.0.to_owned(),
        role: tag_role.1.to_owned(),
        name: name.to_owned(),
        left,
        top,
        width,
        height,
        depth: 0,
        parent,
        across: Overflow::Shows,
        down: Overflow::Shows,
        sampled: Sampled::Words,
        first_mark: -1,
        underlined: false,
        fill: None,
        class: String::new(),
        text: None,
    }
}

/// A box that writes words at 14 px.
fn words(name: &str, at: [i64; 4]) -> Drawn {
    Drawn {
        text: Some(TextRun {
            ink: None,
            px_x100: 1400,
            cut: Cut::Fits,
        }),
        ..el(("P", "-"), name, at, -1)
    }
}

fn crushed(drawn: &[Drawn]) -> Vec<Violation> {
    let mut out = Vec::new();
    no_text_is_crushed(drawn, AT, &mut out);
    out
}

fn clipped(drawn: &[Drawn]) -> Vec<Violation> {
    let mut out = Vec::new();
    every_popover_shows_an_option(drawn, AT, &mut out);
    out
}

#[test]
fn a_sentence_set_one_glyph_to_a_line_is_caught() {
    let found = crushed(&[words("城的原话", [300, 40, 18, 88])]);
    assert_eq!(found.len(), 1, "one crushed sentence, one finding");
    assert!(found[0].location.starts_with(AT));
}

#[test]
fn a_sentence_on_one_line_passes() {
    assert!(crushed(&[words("城的原话", [300, 40, 60, 20])]).is_empty());
}

#[test]
fn a_single_glyph_in_a_narrow_tall_button_is_not_crushed() {
    assert!(crushed(&[words("×", [300, 40, 20, 32])]).is_empty());
}

#[test]
fn a_list_cut_to_less_than_one_option_is_caught() {
    let holder = Drawn {
        down: Overflow::Clips,
        ..el(("DIV", "-"), "-", [0, 0, 300, 50], -1)
    };
    let found = clipped(&[
        holder,
        el(("UL", "listbox"), "models", [0, 40, 300, 64], 0),
        el(("LI", "option"), "fake-small", [0, 40, 300, 32], 1),
    ]);
    assert_eq!(found.len(), 1, "one clipped list, one finding");
}

#[test]
fn a_list_with_room_for_its_options_passes() {
    let holder = Drawn {
        down: Overflow::Scrolls,
        ..el(("DIV", "-"), "-", [0, 0, 300, 400], -1)
    };
    assert!(
        clipped(&[
            holder,
            el(("UL", "listbox"), "models", [0, 40, 300, 64], 0),
            el(("LI", "option"), "fake-small", [0, 40, 300, 32], 1),
        ])
        .is_empty()
    );
}

#[test]
fn a_list_that_spills_past_a_holder_that_shows_it_passes() {
    assert!(
        clipped(&[
            el(("DIV", "-"), "-", [0, 0, 300, 10], -1),
            el(("UL", "listbox"), "models", [0, 40, 300, 64], 0),
            el(("LI", "option"), "fake-small", [0, 40, 300, 32], 1),
        ])
        .is_empty()
    );
}
