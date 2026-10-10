// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! The edges of a frame: what a record cannot make the terminal do, a
//! scene the leaf refuses, the cursor once no live region is drawn, and
//! what gives way first in a narrow or a short window.

use super::{drawn, live, screen, transcript};
use crate::scene::{Entry, Inline, Quiet, Scene};

#[test]
fn a_control_sequence_inside_a_reply_reaches_the_screen_as_a_mark() {
    let entries = vec![Entry::Reply {
        said: "clear\u{1b}[2Jthe screen".to_owned(),
    }];
    let scene = Scene::inline(&Inline {
        columns: 80,
        erase: 0,
        previous: None,
        entries: &entries,
        live: None,
    });
    let (bytes, _) = drawn(&scene);
    let written = String::from_utf8(bytes.clone()).unwrap();
    assert!(
        written.contains("clear\u{fffd}[2Jthe screen"),
        "{written:?}"
    );
    let (shown, _) = screen(80, 4, &[bytes]);
    assert!(shown.contains("the screen"), "{shown}");
}

#[test]
fn a_scene_cut_short_is_refused_rather_than_read_past() {
    let entries = transcript();
    let scene = Scene::inline(&Inline {
        columns: 80,
        erase: 0,
        previous: None,
        entries: &entries,
        live: None,
    });
    let bytes = scene.bytes();
    for cut in [1, 7, bytes.len() / 2, bytes.len() - 1] {
        let short = Scene::from_bytes_for_tests(&bytes[..cut]);
        let mut into = Vec::new();
        assert_eq!(
            crate::draw(&short, &mut into),
            Err(crate::Refused::Malformed)
        );
    }
}

/// A frame with no live region, as the console draws once the terminal
/// is handed back, leaves the cursor shown: every frame hides it while it
/// draws.
#[test]
fn a_frame_without_a_live_region_leaves_the_cursor_shown() {
    let entries = vec![Entry::Note {
        said: "the city exits without a handoff".to_owned(),
    }];
    let scene = Scene::inline(&Inline {
        columns: 80,
        erase: 0,
        previous: None,
        entries: &entries,
        live: None,
    });
    let (bytes, _) = drawn(&scene);
    let mut parser = vt100::Parser::new(4, 80, 0);
    parser.process(&bytes);
    assert!(!parser.screen().hide_cursor());
}

/// In a narrow window the resident's name stays and the time and the key
/// that stops the run give way first.
#[test]
fn a_narrow_working_row_keeps_who_is_working() {
    let mut narrow = live("", 0, None);
    narrow.waiting = None;
    narrow.calling = Vec::new();
    let scene = Scene::inline(&Inline {
        columns: 28,
        erase: 0,
        previous: None,
        entries: &[],
        live: Some(narrow),
    });
    let (bytes, _) = drawn(&scene);
    let (shown, _) = screen(28, 8, &[bytes]);
    assert_eq!(shown.lines().nth(1), Some("● mayor is working"), "{shown}");
}

/// A window too short for the transient keeps the address and the code,
/// and draws no transient over them.
#[test]
fn a_short_window_drops_the_transient_rather_than_cover_the_code() {
    let scene = Scene::quiet(&Quiet {
        columns: 60,
        rows: 2,
        url: "http://127.0.0.1:7341/",
        code: "K7M2QX9P",
        key: None,
        transient: Some("the city is closing"),
    });
    let (bytes, _) = drawn(&scene);
    let (shown, _) = screen(60, 2, &[bytes]);
    assert_eq!(
        shown,
        "                  http://127.0.0.1:7341/\n                  pairing code   K7M2QX9P"
    );
}
