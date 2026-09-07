// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

//! Fragments both ways, through the production door.

use channels::{Address, RunId};

use super::fragments::{from_fragment, to_fragment};
use super::view::{Lens, View};

/// Every view this client has, so the round trip is exhaustive by
/// construction: a variant added without a fragment fails to compile
/// here rather than shipping as a page nobody can link to.
fn every_view() -> Vec<View> {
    let all = [
        View::Sessions,
        View::Session(Address::parse("lab/parser").unwrap()),
        View::Waiting,
        View::Record(Lens::Ledger),
        View::Record(Lens::Archive),
        View::Record(Lens::Bin),
        View::Cost,
        View::Setup,
        View::Building(Address::parse("lab").unwrap()),
        View::Run(RunId::from_bytes([7u8; 16])),
    ];
    // The match is what makes the list exhaustive: adding a variant
    // stops this compiling until the list names it.
    for view in &all {
        match view {
            View::Sessions
            | View::Session(_)
            | View::Waiting
            | View::Record(_)
            | View::Cost
            | View::Setup
            | View::Building(_)
            | View::Run(_) => {}
        }
    }
    all.to_vec()
}

#[test]
fn every_view_survives_the_address_bar_unchanged() {
    for view in every_view() {
        let written = to_fragment(&view);
        assert_eq!(
            from_fragment(&written).as_ref(),
            Some(&view),
            "{written} did not come back as the view that wrote it"
        );
    }
}

#[test]
fn every_fragment_is_absolute_so_a_hand_written_one_matches() {
    for view in every_view() {
        assert!(to_fragment(&view).starts_with("#/"));
    }
}

/// An empty address bar is the sessions list, which is what a person
/// gets by opening the URL the terminal printed.
#[test]
fn nothing_in_the_address_bar_is_the_first_page() {
    for empty in ["", "#", "#/"] {
        assert_eq!(from_fragment(empty), Some(View::Sessions));
    }
}

/// A session is addressed by the name a person gave it, which is the
/// whole reason this page exists: `#/live/<uuid>` named a session by
/// a number nobody chose, so "open yesterday's session" had no
/// answer even after the query behind it was built.
#[test]
fn a_session_is_named_by_its_room_and_not_by_a_number() {
    let addr = Address::parse("lab/parser").unwrap();
    assert_eq!(to_fragment(&View::Session(addr.clone())), "#/s/lab/parser");
    assert_eq!(from_fragment("#/s/lab/parser"), Some(View::Session(addr)));
}

/// Every fragment the previous information architecture wrote still
/// lands, on the page that inherited its question. This is the whole
/// promise; the table is the promise written down.
#[test]
fn every_fragment_the_old_pages_wrote_still_lands() {
    let kept = [
        ("#/overview", View::Sessions),
        ("#/city", View::Sessions),
        ("#/live", View::Sessions),
        ("#/approvals", View::Waiting),
        ("#/ledger", View::Record(Lens::Ledger)),
        ("#/archive", View::Record(Lens::Archive)),
        ("#/recycle-bin", View::Record(Lens::Bin)),
        ("#/dashboard", View::Cost),
        ("#/settings", View::Setup),
    ];
    for (fragment, landing) in kept {
        assert_eq!(
            from_fragment(fragment),
            Some(landing),
            "{fragment} was a link somebody kept"
        );
    }
    // Two that carry an argument, so they cannot go in the table.
    assert_eq!(
        from_fragment("#/building/lab"),
        Some(View::Building(Address::parse("lab").unwrap()))
    );
    assert_eq!(
        from_fragment("#/live/07070707-0707-0707-0707-070707070707"),
        Some(View::Run(RunId::from_bytes([7u8; 16]))),
        "a run named by an old link is resolved to its room by the page, not by the router"
    );
}

/// An old spelling resolves and is never written back, so a person
/// following one lands on the page and then sees its real name.
#[test]
fn no_view_writes_a_fragment_this_build_no_longer_uses() {
    let retired = [
        "#/overview",
        "#/city",
        "#/approvals",
        "#/ledger",
        "#/archive",
        "#/recycle-bin",
        "#/dashboard",
        "#/settings",
        "#/building/lab",
    ];
    for view in every_view() {
        let written = to_fragment(&view);
        assert!(
            !retired.contains(&written.as_str()),
            "{written} is a retired spelling and something still writes it"
        );
    }
}

/// A link that does not resolve says so rather than landing
/// somewhere else quietly.
#[test]
fn a_fragment_that_names_nothing_answers_nothing() {
    for wrong in [
        "#/nowhere",
        "#/s/",
        "#/b/",
        "#/building/",
        "#/live/not-a-run",
        "#/city/extra",
        "#/record/nowhere",
        "#/waiting/extra",
    ] {
        assert_eq!(from_fragment(wrong), None, "{wrong} resolved to something");
    }
}

#[test]
fn the_default_view_answers_the_question_somebody_arrives_with() {
    // A person opening this product is asking "is anything
    // happening, does any of it need me, and how do I start
    // something". One page answers all three, and it is the page the
    // bare fragment lands on.
    assert_eq!(View::default(), View::Sessions);
    assert_eq!(
        crate::route::from_fragment("#/"),
        Some(View::Sessions),
        "the bare fragment and the default view are the same page"
    );
}
