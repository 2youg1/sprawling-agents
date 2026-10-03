// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The golden lines: the bytes `storage::checkpoint::commit` and
//! `sprawling::effect::Landing::shelf` wrote by hand, which the ledgers
//! a city already holds are made of.

use super::*;
use crate::event::Payload;
use crate::locator::Locator;

const SWEPT: &str = r#"{"paths":["file:work/doomed.txt"],"restoration":{"tracked":"file:work/doomed.txt@0123456789abcdef0123456789abcdef01234567"}}"#;
const SHELVED: &str = r#"{"day":20260,"kind":"decision","subject":"parse before you trust"}"#;

fn bytes(payload: &Payload) -> String {
    serde_json::to_string(payload).unwrap()
}

fn line(golden: &str) -> Payload {
    serde_json::from_str(golden).unwrap()
}

#[test]
fn a_swept_file_writes_the_bytes_the_checkpoint_wrote() {
    let tracked =
        Locator::parse("file:work/doomed.txt@0123456789abcdef0123456789abcdef01234567").unwrap();
    let typed = FileDiscarded {
        paths: vec!["file:work/doomed.txt".to_owned()],
        restoration: Some(Restoration::Tracked(tracked)),
    };
    assert_eq!(bytes(&Payload::of(&typed).unwrap()), SWEPT);
    assert_eq!(line(SWEPT).read::<FileDiscarded>().unwrap(), typed);
}

#[test]
fn a_restoration_this_build_cannot_name_leaves_the_paths_readable() {
    let read = line(r#"{"paths":["file:a"],"restoration":{"buried":"x"}}"#)
        .read::<FileDiscarded>()
        .unwrap();
    assert_eq!(
        read,
        FileDiscarded {
            paths: vec!["file:a".to_owned()],
            restoration: None,
        }
    );
}

#[test]
fn a_shelved_entry_writes_the_bytes_the_shelf_wrote() {
    let typed = AssetArchived {
        kind: "decision".to_owned(),
        day: 20260,
        subject: "parse before you trust".to_owned(),
    };
    assert_eq!(bytes(&Payload::of(&typed).unwrap()), SHELVED);
    assert_eq!(line(SHELVED).read::<AssetArchived>().unwrap(), typed);
}

/// An entry that names no kind is a fact, the kind an unmarked entry has
/// always been shown as.
#[test]
fn an_entry_that_names_no_kind_is_a_fact() {
    assert_eq!(
        line(r#"{"day":3,"subject":"s"}"#)
            .read::<AssetArchived>()
            .unwrap(),
        AssetArchived {
            kind: "fact".to_owned(),
            day: 3,
            subject: "s".to_owned(),
        }
    );
}
