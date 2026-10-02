// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A browser is an engine, and one machine's brand of it
//! (`crates/sprawling/spec/Doctor.lean` §8-57).

use std::collections::BTreeSet;

use crate::doctor::family::{Confidence, Family, member_at};
use crate::doctor::{Detection, Platform, REQUIREMENTS, Tier};

/// Every family is a row of the tier that runs a city, and every such
/// row that detects a family is a family: the two lists cannot drift
/// apart, because the name a capability tries is the name the table
/// carries. The develop tier's render browser is a second reason to
/// have a Chromium browser, not a fourth engine.
#[test]
fn every_family_is_a_row_and_every_member_says_where_it_is_installed() {
    let detected: BTreeSet<&str> = REQUIREMENTS
        .iter()
        .filter(|item| item.tier == Tier::Use && matches!(item.detect, Detection::Family(_)))
        .map(|item| item.name)
        .collect();
    assert_eq!(
        detected,
        ["gecko", "chromium", "webkit"].into_iter().collect(),
        "the three engines a session can be held with are three rows"
    );
    for family in Family::ALL {
        for member in family.members() {
            let places: usize = Platform::ALL
                .into_iter()
                .map(|platform| member.places.at(platform).len())
                .sum();
            assert!(
                places > 0,
                "{} says nowhere it is installed, so only a search path finds it",
                member.name
            );
            for platform in Platform::ALL {
                // Judged by spelling rather than by `is_absolute`,
                // which answers about the platform running the test and
                // not about the platform the column describes.
                for place in member.places.at(platform).iter() {
                    assert!(
                        place.starts_with('/') || place.contains(":\\"),
                        "{} names {place}, which is not a place",
                        member.name
                    );
                }
            }
        }
    }
}

/// A brand is found by its own installed path before its program name,
/// because two brands can ship a program under one name: Firefox
/// Developer Edition's is called `firefox`, and only its directory
/// tells the two apart.
#[test]
fn a_found_browser_is_reported_by_the_brand_it_is() {
    let developer = std::path::Path::new(r"C:\Program Files\Firefox Developer Edition\firefox.exe");
    assert_eq!(
        member_at(Family::Gecko, developer).map(|member| member.name),
        Some("firefox-developer")
    );
    assert_eq!(
        member_at(Family::Gecko, std::path::Path::new("/usr/bin/zen")).map(|member| member.name),
        Some("zen")
    );
    assert_eq!(
        member_at(
            Family::Chromium,
            std::path::Path::new("/usr/local/bin/vivaldi")
        )
        .map(|member| member.name),
        Some("vivaldi"),
        "a brand installed where this table does not look is still that brand"
    );
}

/// A browser's version is read out of the files beside it and never
/// asked of the browser (`crates/sprawling/spec/Doctor.lean` §8-80).
///
/// On Windows a Chromium browser given `--version` opens a window
/// instead of printing a line, and when one is already running the new
/// process hands the request over and exits, so the window outlives the
/// child this city can stop. `family` is the one file that could bring
/// that spawn back, and this holds it to reading rather than running.
#[test]
fn no_browser_is_started_to_learn_its_version() {
    let looking = include_str!("../family.rs");
    for spelling in ["ask_version", "Command::new"] {
        assert!(
            !looking.contains(spelling),
            "doctor::family names {spelling}, so looking for a browser can start one"
        );
    }
}

/// The two members this project cannot simply promise are last in their
/// families, so a machine with an ordinary browser never answers with
/// the awkward one.
#[test]
fn a_member_that_needs_confirming_is_never_the_first_answer() {
    let gecko = Family::Gecko.members();
    let tor = gecko
        .iter()
        .position(|member| member.name == "tor-browser")
        .expect("the Gecko family carries Tor Browser");
    assert_eq!(
        tor,
        gecko.len().saturating_sub(1),
        "Tor Browser answers only when nothing else does"
    );
    assert_eq!(
        gecko
            .get(tor)
            .map(|member| member.confidence.caveat())
            .unwrap_or_default(),
        ", confirm it by hand once"
    );
    assert!(
        Family::WebKit
            .members()
            .iter()
            .all(|member| member.confidence == Confidence::Experimental),
        "Safari's BiDi support is partial, and the report says so"
    );
}

/// `cargo xtask render` finds its engine through `chromium.tsv`, which
/// is this family rendered, so the gate and the doctor look in the same
/// places for the same brands; when they differ this prints the file
/// the family wants.
#[test]
fn the_chromium_file_is_the_family_rendered() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/doctor/family/chromium.tsv");
    let rendered = Family::Chromium.members().iter().fold(
        concat!(
            "# generated from crates/sprawling/src/doctor/family/chromium.rs by the test ",
            "`the_chromium_file_is_the_family_rendered`; edit the family, not this file\n",
            "# brand<TAB>program<TAB>windows<TAB>macos<TAB>linux, places separated by |\n"
        )
        .to_owned(),
        |mut file, member| {
            let places = |platform| member.places.at(platform).join("|");
            file.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                member.name,
                member.program,
                places(Platform::Windows),
                places(Platform::MacOs),
                places(Platform::Linux)
            ));
            file
        },
    );
    let written = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        written == rendered,
        "{} is not what the family renders; write this into it:\n<<<\n{rendered}>>>",
        path.display()
    );
}
