// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a model reads is the same bytes whichever way the engine got
//! them (`crates/runtime/spec/Tools/Read.lean` D10): the handle read
//! answers what the judged path answers, success and refusal alike, and
//! the one-pass cut cuts what the cut it replaced cut.

use proptest::prelude::*;

use super::*;
use crate::tools::chosen_path::{CityRoot, make_link};

/// The cut `Interval::cut` replaced, as it stood: every line collected,
/// then sliced. Kept verbatim as the reference the one-pass cut answers
/// to.
pub(super) fn reference_cut(
    offset: u64,
    limit: u16,
    text: &str,
) -> (String, Option<usize>, Option<usize>) {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let total = lines.len();
    let start = match usize::try_from(offset) {
        Ok(start) if start < total => start,
        Ok(_) | Err(_) => return (String::new(), Some(total), None),
    };
    let ceiling = start.saturating_add(usize::from(limit)).min(total);
    let budget = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let mut end = start;
    let mut spent = 0usize;
    while end < ceiling {
        let Some(line) = lines.get(end) else { break };
        let after = spent.saturating_add(line.len());
        if after > budget && end > start {
            break;
        }
        spent = after;
        end = end.saturating_add(1);
    }
    let taken = lines.get(start..end).unwrap_or_default().concat();
    if end < total {
        (taken, Some(total), Some(end))
    } else {
        (taken, None, None)
    }
}

/// Texts that reach both ceilings: short lines in their hundreds, and
/// lines long enough that a few of them pass the byte budget.
fn texts() -> impl Strategy<Value = String> {
    let line = prop_oneof![
        8 => "[a-z ]{0,12}",
        1 => (0usize..40_000).prop_map(|len| "x".repeat(len)),
    ];
    (
        proptest::collection::vec(line, 0..700),
        any::<bool>(),
        "\r?",
    )
        .prop_map(|(lines, ends_open, cr)| {
            let mut text = lines.join(&format!("{cr}\n"));
            if !ends_open && !text.is_empty() {
                text.push('\n');
            }
            text
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn the_cut_agrees_with_the_one_it_replaced(
        text in texts(),
        offset in prop_oneof![0u64..800, Just(u64::MAX)],
        limit in 1u16..=512,
    ) {
        let taken = Interval { offset, limit }.cut(&text);
        prop_assert_eq!(
            (taken.text, taken.total_lines, taken.next_offset),
            reference_cut(offset, limit, &text)
        );
    }

    #[test]
    fn a_handle_read_answers_what_the_judged_path_answers(
        bytes in proptest::collection::vec(any::<u8>(), 0..4096),
        text in "[a-z\n\r]{0,4096}",
        raw in any::<bool>(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("lab")).unwrap();
        let content = if raw { bytes } else { text.into_bytes() };
        std::fs::write(dir.path().join("lab").join("file.txt"), content).unwrap();
        let answers = both(dir.path(), open_everywhere(), "lab/file.txt");
        prop_assert_eq!(&answers.0, &answers.1);
    }
}

fn open_everywhere() -> ReadBound {
    Arc::new(|_: &kernel::Address| kernel::ReadVerdict::Open)
}

/// The engine's answer and the judged path's answer to one admitted
/// path, each printed whole, so a refusal compares its code, message,
/// recovery and `nearby` together.
fn both(root: &Path, bound: ReadBound, asked: &str) -> (String, String) {
    let city = CityRoot::anchor(root, "read").unwrap();
    let addr = kernel::Address::parse(asked).unwrap();
    let print = |answer: Result<super::engine::Looked, AxError>| match answer {
        Ok((text, _, _)) => format!("ok {text:?}"),
        Err(err) => format!("err {}", serde_json::to_string(&err).unwrap()),
    };
    (
        print(super::engine::text_of(&city, &addr, asked, &*bound)),
        print(super::engine::judged_read(&city, &addr, asked, &*bound)),
    )
}

/// The places a read refuses or answers through a link, each answered
/// alike by both ways.
#[test]
fn every_door_answers_alike_both_ways() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for room in ["lab/room", "vault/room1", "annex"] {
        std::fs::create_dir_all(dir.path().join(room)).unwrap();
    }
    std::fs::write(dir.path().join("lab/room/notes.md"), "notes\n").unwrap();
    std::fs::write(dir.path().join("lab/room/latin1.txt"), [0xE9, b'\n']).unwrap();
    std::fs::write(dir.path().join("vault/room1/secret.md"), "the vault\n").unwrap();
    std::fs::write(dir.path().join("annex/plan.md"), "the annex\n").unwrap();
    std::fs::write(outside.path().join("secret.md"), "elsewhere\n").unwrap();
    let lab = dir.path().join("lab");
    make_link(
        &lab.join("to-vault"),
        &dir.path().join("vault").join("room1"),
    );
    make_link(&lab.join("to-annex"), &dir.path().join("annex"));
    make_link(&lab.join("out"), outside.path());
    let vault_closed: ReadBound = Arc::new(|addr: &kernel::Address| {
        if addr.as_str().starts_with("vault") {
            kernel::ReadVerdict::Confidential
        } else {
            kernel::ReadVerdict::Open
        }
    });
    let asked = [
        "lab/room/notes.md",
        "lab/room/latin1.txt",
        "lab/room",
        "lab/room/absent.md",
        "lab/absent/deeper.md",
        "lab/to-vault/secret.md",
        "lab/to-annex/plan.md",
        "lab/to-annex/absent.md",
        "lab/out/secret.md",
    ];
    let (by_handle, judged): (Vec<String>, Vec<String>) = asked
        .iter()
        .map(|asked| both(dir.path(), Arc::clone(&vault_closed), asked))
        .unzip();
    assert_eq!(by_handle, judged);
}
