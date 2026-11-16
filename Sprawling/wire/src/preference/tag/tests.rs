// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::{PreferencePatch, PreferencesAnswer};

fn tagged(room: &str, began: u64, tags: &[&str]) -> SessionTags {
    SessionTags {
        city: Address::parse("harbour").unwrap(),
        room: Address::parse(room).unwrap(),
        began: Seq::new(began),
        tags: tags.iter().map(|tag| Tag::parse(tag).unwrap()).collect(),
    }
}

/// A tag is one word of any script; whitespace, punctuation past `-`
/// and `_`, and a word past the cap are refused where they are read, so
/// a file a person edited by hand cannot hold a tag the page cannot draw.
#[test]
fn a_tag_is_one_word_of_letters_digits_dash_or_underscore() {
    for good in [
        "bug",
        "重构",
        "v2",
        "follow-up",
        "two_words",
        &"x".repeat(TAG_MAX),
    ] {
        assert!(Tag::parse(good).is_ok(), "{good}");
    }
    for bad in [
        "",
        "two words",
        "a/b",
        "#x",
        "tab\t",
        &"x".repeat(TAG_MAX + 1),
    ] {
        assert!(Tag::parse(bad).is_err(), "{bad:?}");
    }
    assert!(serde_json::from_str::<Tag>(r#""two words""#).is_err());
}

/// The patch states a session's whole set: it replaces what was held,
/// an empty set removes the entry, and the list is kept in session order
/// with each set sorted and without repeats, whatever order it arrived in.
#[test]
fn a_tags_patch_replaces_one_sessions_set_and_keeps_the_list_ordered() {
    let mut held = PreferencesAnswer::default();
    held.apply(PreferencePatch::Tags(tagged("lab/b", 9, &["x"])));
    held.apply(PreferencePatch::Tags(tagged(
        "hall/mayor",
        3,
        &["pin", "bug", "pin"],
    )));
    held.apply(PreferencePatch::Tags(tagged("lab/b", 9, &["y"])));
    assert_eq!(
        held.tags,
        vec![
            tagged("hall/mayor", 3, &["bug", "pin"]),
            tagged("lab/b", 9, &["y"])
        ]
    );
    held.apply(PreferencePatch::Tags(tagged("hall/mayor", 3, &[])));
    assert_eq!(held.tags, vec![tagged("lab/b", 9, &["y"])]);
}

/// The same room and line in another city is another session.
#[test]
fn the_city_is_part_of_a_sessions_name() {
    let mut held = PreferencesAnswer::default();
    let other = SessionTags {
        city: Address::parse("fjord").unwrap(),
        ..tagged("hall/mayor", 3, &["b"])
    };
    held.apply(PreferencePatch::Tags(tagged("hall/mayor", 3, &["a"])));
    held.apply(PreferencePatch::Tags(other.clone()));
    assert_eq!(held.tags, vec![other, tagged("hall/mayor", 3, &["a"])]);
}
