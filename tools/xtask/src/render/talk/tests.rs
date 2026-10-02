// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

use browser::survey::Sampled;

fn words() -> Words {
    Words {
        conversation: vec!["conversation".to_owned(), "对话".to_owned()],
        edge: vec!["edge".to_owned()],
        inspect: vec!["inspect".to_owned()],
    }
}

/// One element: tag, role, name, and where it stands as its top edge and
/// its nearest measured ancestor. Every box is 40 px tall and wide enough
/// to be drawn.
fn el(tag: &str, role: &str, name: &str, (top, parent): (i64, i64)) -> Drawn {
    Drawn {
        across: Overflow::Shows,
        down: Overflow::Shows,
        sampled: Sampled::Frame,
        first_mark: -1,
        underlined: false,
        fill: None,
        class: String::new(),
        text: None,
        tag: tag.to_owned(),
        role: role.to_owned(),
        name: name.to_owned(),
        left: 100,
        top,
        width: 200,
        height: 40,
        depth: 0,
        parent,
    }
}

/// The shell fixture as the probe reads it: a case holding the
/// conversation and the edge keys; a thread that scrolls, with one
/// control in a message; the composer's text box, coin key and one chip;
/// and three edge keys.
fn shell() -> Vec<Drawn> {
    let mut thread = el("DIV", "-", "-", (40, 1));
    thread.down = Overflow::Scrolls;
    vec![
        el("SECTION", "-", "shell", (0, -1)),
        el("SECTION", "-", "对话", (0, 0)),
        thread,
        el("BUTTON", "-", "read src/main.rs", (60, 2)),
        el("TEXTAREA", "-", "write to ledger", (800, 1)),
        el("BUTTON", "-", "send", (800, 1)),
        el("BUTTON", "-", "release/ledger", (840, 1)),
        el("NAV", "-", "edge", (700, 0)),
        el("BUTTON", "-", "layers", (700, 7)),
        el("BUTTON", "-", "mailbox", (750, 7)),
        el("A", "-", "settings", (800, 7)),
    ]
}

fn judged(drawn: &[Drawn]) -> (Option<u64>, Vec<String>) {
    let mut out = Vec::new();
    let counted =
        the_conversation_page_holds_its_controls(drawn, "gallery 1280", &words(), &mut out);
    (
        counted,
        out.into_iter().map(|found| found.violation).collect(),
    )
}

#[test]
fn the_three_clusters_are_counted_and_a_control_in_a_message_is_not() {
    assert_eq!(judged(&shell()), (Some(6), Vec::new()));
}

#[test]
fn a_bar_standing_above_the_composer_is_refused_and_a_notice_is_not() {
    let mut barred = shell();
    barred.push(el("BUTTON", "-", "share", (10, 1)));
    let (_, found) = judged(&barred);
    assert_eq!(
        found,
        [
            "button `share` at x=100 y=10 neither scrolls with the thread nor belongs to the composer"
        ]
    );

    let mut noticed = shell();
    noticed.push(el("DIV", "status", "-", (700, 1)));
    noticed.push(el("BUTTON", "-", "reconnect", (700, 11)));
    assert_eq!(judged(&noticed), (Some(6), Vec::new()));
}

#[test]
fn the_inspect_side_counts_when_it_is_open() {
    let mut open = shell();
    open.push(el("ASIDE", "-", "inspect", (0, 0)));
    open.push(el("BUTTON", "-", "close", (0, 11)));
    assert_eq!(judged(&open).0, Some(7));
}

#[test]
fn a_conversation_drawn_without_the_edge_keys_is_not_a_page() {
    let alone: Vec<Drawn> = shell().into_iter().take(7).collect();
    assert_eq!(judged(&alone), (None, Vec::new()));
}

#[test]
fn the_register_holds_the_count_and_a_gallery_without_a_page_is_refused() {
    let register: toml::Value =
        toml::from_str("[talk_controls]\nbest_count = 6\nslack_count = 0\n").unwrap();
    let held = |counted: Option<u64>| {
        let mut out = Vec::new();
        talk_controls_within_register(counted, &register, "gallery 1280", &mut out);
        out.into_iter()
            .map(|found| found.violation)
            .collect::<Vec<_>>()
    };
    assert_eq!(held(Some(6)), Vec::<String>::new());
    assert_eq!(held(Some(5)), Vec::<String>::new());
    assert_eq!(
        held(Some(7)),
        [
            "the most crowded conversation page stands 7 controls; the register accepted 6 and 0 more"
        ]
    );
    assert_eq!(held(None), ["the gallery draws no conversation page"]);
}
