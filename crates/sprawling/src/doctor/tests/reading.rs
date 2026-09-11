// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person reads off the report: four status words, two sections,
//! colour a terminal may refuse, and a version column no vendor banner
//! can push off the screen (sprawling-SPEC.md section 8-59).

use super::{ScriptedMachine, finding_for};
use crate::doctor::paint::{Counted, Ink, Part, Status, count, row, summary};
use crate::doctor::{Fault, Finding, Presence, REQUIREMENTS, Tier, Version, examine};

/// Four status words, one per state, so a person reads the column
/// rather than the sentence - and an item nobody is waiting for is
/// never read as an item standing in the way.
#[test]
fn one_row_per_item_carries_one_of_four_status_words() {
    let present = finding_for("git", &[]);
    assert_eq!(Status::of(&present), Status::Present);
    let line = row(&present, Ink::Plain);
    assert!(line.contains("present"), "{line}");
    assert!(line.contains("9.9.9 at /bin/git"), "{line}");

    let absent = finding_for("git", &["git"]);
    assert_eq!(Status::of(&absent), Status::Missing);
    assert!(
        row(&absent, Ink::Plain)
            .trim()
            .ends_with("not on the search path")
    );

    let optional = finding_for("ffmpeg", &["ffmpeg"]);
    assert_eq!(Status::of(&optional), Status::Optional);
    let line = row(&optional, Ink::Plain);
    assert!(line.contains("optional"), "{line}");
    assert!(line.contains("it enables"), "{line}");

    let broken = Finding {
        requirement: REQUIREMENTS
            .iter()
            .find(|item| item.name == "git")
            .expect("the table carries git"),
        presence: Presence::Broken {
            at: std::path::PathBuf::from("/bin/git"),
            fault: Fault::WillNotStart("permission denied".to_owned()),
        },
    };
    assert_eq!(Status::of(&broken), Status::Broken);
    assert!(row(&broken, Ink::Plain).contains("broken"));
}

/// Colour is an offer a terminal may refuse, and either refusal is
/// enough: the environment's and the command line's.
#[test]
fn no_color_is_honoured_from_the_environment_and_from_the_flag() {
    let present = finding_for("git", &[]);
    assert!(row(&present, Ink::Colour).contains('\u{1b}'));
    assert!(!row(&present, Ink::Plain).contains('\u{1b}'));
    assert_eq!(
        Ink::Colour.or_plain(Some(std::ffi::OsString::new())),
        Ink::Plain,
        "NO_COLOR is a request whatever it is set to, including nothing"
    );
    let words: Vec<String> = ["doctor", "--no-color"]
        .iter()
        .map(|word| (*word).to_owned())
        .collect();
    assert_eq!(crate::doctor::screen::asked(&words, None).ink, Ink::Plain);
}

/// A version column a vendor's banner cannot push off the screen.
#[test]
fn a_version_is_the_number_out_of_whatever_the_tool_printed() {
    let said = |text: &str| Version::Said(text.to_owned()).number();
    assert_eq!(
        said("ffmpeg version N-125649-g8d3942 Copyright (c) 2000-2026 the FFmpeg developers"),
        "N-125649-g8d"
    );
    assert_eq!(said("Mozilla Firefox 133.0.3"), "133.0.3");
    assert_eq!(said("git version 2.43.0"), "2.43.0");
    assert_eq!(said("cargo-nextest-cargo-nextest 0.9.78"), "0.9.78");
    assert_eq!(
        Version::Silent.number(),
        "said nothing",
        "a tool that printed nothing says so here too"
    );
}

/// The two sections a person reads: what stands between them and a
/// running city, and what each other item would add.
#[test]
fn the_report_is_grouped_into_required_and_recommended() {
    let findings = examine(&ScriptedMachine::missing(&[]));
    let required: Vec<&str> = findings
        .iter()
        .filter(|found| Part::of(found) == Part::Required)
        .map(|found| found.requirement.name)
        .collect();
    assert_eq!(
        required,
        vec!["gecko", "webkit", "chromedriver", "msedgedriver"],
        "required is the use tier's own items; everything else is a recommendation"
    );
    assert!(
        findings
            .iter()
            .filter(|found| found.requirement.tier == Tier::Develop)
            .all(|found| Part::of(found) == Part::Recommended),
        "a person who only wants to run a city is not told they are short a fuzzer"
    );
}

/// The closing count agrees with the verdict above it: a family of
/// browsers is one thing to have, so a machine with one of them reads
/// `1 / 1` beside `ready to use` rather than `1 / 4`.
#[test]
fn a_family_of_browsers_counts_once_in_the_summary() {
    let only_zen = examine(&ScriptedMachine::missing(&[
        "webkit",
        "chromedriver",
        "msedgedriver",
    ]));
    assert_eq!(
        count(&only_zen, Part::Required),
        Counted { here: 1, wanted: 1 }
    );
    assert!(
        summary(&only_zen, Ink::Plain)
            .iter()
            .any(|line| line.contains("next: sprawling up")),
        "a person who has what a city needs is told the command that starts one"
    );

    let no_engine = examine(&ScriptedMachine::missing(&[
        "gecko",
        "webkit",
        "chromedriver",
        "msedgedriver",
    ]));
    assert_eq!(
        count(&no_engine, Part::Required),
        Counted { here: 0, wanted: 1 }
    );
    assert!(
        summary(&no_engine, Ink::Plain)
            .iter()
            .any(|line| line.contains("doctor --install")),
        "a person who is short of one is told the command that offers it"
    );
}
