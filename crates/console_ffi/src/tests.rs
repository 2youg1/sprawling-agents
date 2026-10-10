// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

//! The leaf judged by the screen a person sees: every frame is written
//! into a terminal emulator (`vt100`) and the rows it shows are compared
//! whole, so a test reads like the screen it checks.

use crate::part::{Ending, Outcome, Part, Verdict};
use crate::scene::{
    Calling, Choice, Composer, Entry, Inline, Live, Menu, Quiet, Scene, TimeOfDay, Waiting, Working,
};

fn at(h: u32, m: u32, s: u32) -> Option<TimeOfDay> {
    TimeOfDay::from_seconds(h * 3600 + m * 60 + s)
}

/// The rows a terminal of this size shows after `frames`, trailing
/// spaces cut, and the cursor's place.
fn screen(columns: u16, rows: u16, frames: &[Vec<u8>]) -> (String, (u16, u16)) {
    let mut parser = vt100::Parser::new(rows, columns, 0);
    for frame in frames {
        parser.process(frame);
    }
    let screen = parser.screen();
    let shown: Vec<String> = screen
        .rows(0, columns)
        .map(|row| row.trim_end().to_owned())
        .collect();
    let text = shown.join("\n").trim_end().to_owned();
    (text, screen.cursor_position())
}

fn drawn(scene: &Scene) -> (Vec<u8>, crate::Frame) {
    let mut into = Vec::new();
    let frame = crate::draw(scene, &mut into).unwrap();
    (into, frame)
}

fn transcript() -> Vec<Entry> {
    vec![
        Entry::Banner {
            city: "~/cities/first".to_owned(),
            address: "127.0.0.1:7341".to_owned(),
            url: "http://127.0.0.1:7341/".to_owned(),
        },
        Entry::You {
            at: at(1, 24, 31),
            said: "the readings report double-counts August; find where and fix it".to_owned(),
        },
        Entry::Head {
            at: at(1, 24, 33),
            resident: "mayor".to_owned(),
            facts: vec!["deepseek-flash".to_owned(), "work".to_owned()],
        },
        Entry::Reasoning { characters: 1521 },
        Entry::Tool {
            at: at(1, 24, 35),
            name: "read".to_owned(),
            subject: "lab/report/data/readings.csv".to_owned(),
            took_us: Some(1_076),
            outcome: Outcome::Answered,
        },
        Entry::Tool {
            at: at(1, 24, 38),
            name: "exec".to_owned(),
            subject: "python scripts/summary.py --month 08".to_owned(),
            took_us: Some(2_606_000),
            outcome: Outcome::Failed,
        },
        Entry::Reply {
            said: "summary.py groups by `city` and then by `month`, but the August rows are read twice: `load()` appends the cached frame to the fresh one instead of replacing it.".to_owned(),
        },
        Entry::Tool {
            at: at(1, 24, 44),
            name: "edit".to_owned(),
            subject: "lab/report/scripts/summary.py".to_owned(),
            took_us: Some(842),
            outcome: Outcome::Answered,
        },
        Entry::Resolved {
            at: at(1, 24, 50),
            verdict: Verdict::Approved,
            what: "exec  python scripts/summary.py --month 08".to_owned(),
        },
        Entry::Ended {
            at: at(1, 25, 15),
            ending: Ending::Done,
            took_s: Some(44),
        },
    ]
}

fn live<'a>(typed: &'a str, cursor: usize, menu: Option<Menu<'a>>) -> Live<'a> {
    Live {
        waiting: Some(Waiting {
            what: "exec  cargo test -p report",
            more: 0,
        }),
        working: Some(Working {
            resident: "mayor",
            since: at(1, 25, 20),
        }),
        calling: vec![Calling {
            name: "exec",
            subject: "cargo test -p report",
        }],
        asking: None,
        composer: Composer {
            typed,
            cursor,
            placeholder: "to the Mayor…",
            room: "hall/mayor",
            offer: "deepseek-flash · high",
        },
        menu,
    }
}

#[test]
fn a_session_reads_as_one_left_edge_with_times_beside_it() {
    let entries = transcript();
    let scene = Scene::inline(&Inline {
        columns: 80,
        erase: 0,
        previous: None,
        entries: &entries,
        live: Some(live("make the August total a test too", 32, None)),
    });
    let (bytes, frame) = drawn(&scene);
    let (shown, cursor) = screen(80, 34, &[bytes]);
    let wanted = [
        "sprawling  ~/cities/first                   127.0.0.1:7341  /web opens the page",
        "",
        "01:24:31  › the readings report double-counts August; find where and fix it",
        "",
        "01:24:33  ● mayor  deepseek-flash · work",
        "            ▸ reasoning 1,521 characters",
        "01:24:35  ◇ read     lab/report/data/readings.csv                1.0 ms",
        "01:24:38  ◇ exec     python scripts/summary.py --month 08         2.6 s  failed",
        "",
        "            summary.py groups by `city` and then by `month`, but the August",
        "            rows are read twice: `load()` appends the cached frame to the fresh",
        "            one instead of replacing it.",
        "",
        "01:24:44  ◇ edit     lab/report/scripts/summary.py               842 µs",
        "01:24:50  ✓ approved  exec  python scripts/summary.py --month 08",
        "01:25:15  ■ done · 44 s",
        "",
        "          ? exec  cargo test -p report                       y approve   n deny",
        "          ● mayor is working · since 01:25:20                      esc stops it",
        "          ◇ exec     cargo test -p report                       running",
        "",
        "          make the August total a test too",
        "          ────────────────────────────────╌╌╌┄┄┄",
        "          hall/mayor                                      deepseek-flash · high",
    ]
    .join("\n");
    println!("{shown}");
    assert_eq!(shown, wanted);
    assert_eq!(cursor, (21, 42));
    assert_eq!(frame.cursor_row, 5);
}

#[test]
fn the_next_frame_erases_the_live_region_and_writes_under_the_transcript() {
    let entries = transcript();
    let first = Scene::inline(&Inline {
        columns: 80,
        erase: 0,
        previous: None,
        entries: &entries,
        live: Some(live("", 0, None)),
    });
    let (first_bytes, frame) = drawn(&first);
    let more = vec![Entry::Note {
        said: "plain lines go to lab/report".to_owned(),
    }];
    let second = Scene::inline(&Inline {
        columns: 80,
        erase: frame.cursor_row,
        previous: Some(Part::Ended),
        entries: &more,
        live: Some(Live {
            waiting: None,
            working: None,
            calling: Vec::new(),
            asking: None,
            composer: Composer {
                typed: "",
                cursor: 0,
                placeholder: "to report…",
                room: "lab/report",
                offer: "deepseek-flash",
            },
            menu: None,
        }),
    });
    let (second_bytes, _) = drawn(&second);
    let (shown, _) = screen(80, 34, &[first_bytes, second_bytes]);
    let tail: Vec<&str> = shown.lines().rev().take(6).collect();
    let wanted = vec![
        "          lab/report                                             deepseek-flash",
        "          ──╌╌╌┄┄┄",
        "          to report…",
        "",
        "            plain lines go to lab/report",
        "01:25:15  ■ done · 44 s",
    ];
    println!("{shown}");
    assert_eq!(tail, wanted);
}

#[test]
fn the_slash_menu_takes_the_place_of_the_settings_row() {
    let menu = Menu {
        shown: vec![
            Choice {
                spelling: "/web",
                takes: "",
                summary: "open the WebUI and quiet this terminal; Esc comes back",
            },
            Choice {
                spelling: "/wire",
                takes: "<verb> [<json>]",
                summary: "send any wire command or question, its body as JSON",
            },
        ],
        chosen: 0,
        more: 0,
    };
    let scene = Scene::inline(&Inline {
        columns: 80,
        erase: 0,
        previous: None,
        entries: &[],
        live: Some(Live {
            waiting: None,
            working: None,
            calling: Vec::new(),
            asking: None,
            composer: Composer {
                typed: "/w",
                cursor: 2,
                placeholder: "",
                room: "hall/mayor",
                offer: "",
            },
            menu: Some(menu),
        }),
    });
    let (bytes, frame) = drawn(&scene);
    let (shown, cursor) = screen(80, 10, &[bytes]);
    let wanted = [
        "",
        "          /w",
        "          ──╌╌╌┄┄┄",
        "          ▎/web                    open the WebUI and quiet this terminal; Esc…",
        "           /wire <verb> [<json>]   send any wire command or question, its body…",
        "           tab moves · enter chooses · esc closes",
    ]
    .join("\n");
    println!("{shown}");
    assert_eq!(shown, wanted);
    assert_eq!(cursor, (1, 12));
    assert_eq!(frame.cursor_row, 1);
}

#[test]
fn a_narrow_window_drops_the_times_and_keeps_every_line_inside_it() {
    let entries = transcript();
    let scene = Scene::inline(&Inline {
        columns: 40,
        erase: 0,
        previous: None,
        entries: &entries,
        live: Some(live("一二三四五六七八九十一二三四五六七八九十", 20, None)),
    });
    let (bytes, _) = drawn(&scene);
    let (shown, _) = screen(40, 60, &[bytes]);
    println!("{shown}");
    for row in shown.lines() {
        assert!(row.chars().count() <= 39, "{row}");
        assert!(!row.starts_with("01:"), "{row}");
    }
}

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
fn the_quiet_host_is_two_lines_in_the_middle_of_the_window() {
    let scene = Scene::quiet(&Quiet {
        columns: 80,
        rows: 12,
        url: "http://127.0.0.1:7341/",
        code: "K7M2QX9P",
        key: None,
        transient: Some("the city is closing; the runs under way finish first"),
    });
    let (bytes, _) = drawn(&scene);
    let (shown, _) = screen(80, 12, &[bytes]);
    let wanted = [
        "",
        "",
        "",
        "",
        "",
        "              http://127.0.0.1:7341/",
        "              pairing code   K7M2QX9P",
        "",
        "              the city is closing; the runs under way finish first",
    ]
    .join("\n");
    println!("{shown}");
    assert_eq!(shown, wanted);
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
