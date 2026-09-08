// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn sending_work_needs_a_room_and_a_task_and_nothing_else() {
    assert!(dispatch_to_building("lab", "fix the timer", "the test passes").is_some());
    assert!(
        dispatch_to_building("lab", "  ", "the test passes").is_none(),
        "a run with nothing to do is not a command"
    );
    assert!(
        dispatch_to_building("lab", "fix the timer", "").is_some(),
        "an empty goal is how this city spells a conversation, not a missing field"
    );
    assert!(
        dispatch_to_building("", "fix the timer", "the test passes").is_none(),
        "there is no building called nothing"
    );
}

#[test]
fn work_is_sent_to_a_room_and_never_to_a_buildings_root() {
    let Some(ClientFrame::Command(command)) =
        dispatch_to_building("lab", "fix the timer", "the test passes")
    else {
        panic!("a complete form is a command");
    };
    let channels::WireCommand::Dispatch { addr, session, .. } = *command else {
        panic!("the send-work form makes a dispatch");
    };
    assert_eq!(addr.as_str(), "lab");
    // The room is opened by the city from this name, and the name is
    // the work rather than a counter. Every dispatch from this page
    // used to go to `room1`, so the second piece of work started
    // from a tower wrote over the first one's files.
    let named = session.expect(
        "without a name the city has nothing to open a room from, and the run would hold the \
         whole building's write domain",
    );
    assert_eq!(named.as_str(), "fix the timer");

    let Some(ClientFrame::Command(second)) = dispatch_to_building(
        "lab",
        "fix the timer again, and this time read the failing case first",
        "the test passes",
    ) else {
        panic!("a complete form is a command");
    };
    let channels::WireCommand::Dispatch { session, .. } = *second else {
        panic!("the send-work form makes a dispatch");
    };
    assert_eq!(
        session.map(|name| name.as_str().to_owned()),
        Some("fix the timer again,".to_owned()),
        "a long task still yields a name short enough to be a folder"
    );
}
