// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Consent is asked one item at a time: a no installs nothing, and the
//! default checks and installs nothing.

use super::ScriptedMachine;
use crate::doctor::paint::Ink;
use crate::doctor::screen::{Asked, run};
use crate::doctor::{Platform, REQUIREMENTS};

/// Consent is asked item by item, and a no installs nothing.
///
/// The command is on the screen before the question, so nobody agrees to
/// a command they were not shown.
#[test]
fn nothing_is_installed_without_a_yes_to_that_one_item() {
    let Some(platform) = Platform::current() else {
        return;
    };
    let machine = ScriptedMachine::missing(&["just"]);
    let mut refused = std::io::Cursor::new(b"n\n".to_vec());
    let mut screen: Vec<u8> = Vec::new();
    let ready = run(
        &Asked {
            install: true,
            city: None,
            scanned: std::env::temp_dir(),
            explain: None,
            ink: Ink::Plain,
        },
        &machine,
        &mut refused,
        &mut screen,
    )
    .unwrap();
    assert!(!ready, "a missing required item is not ready");
    let shown = String::from_utf8(screen).unwrap();
    let command = REQUIREMENTS
        .iter()
        .find(|item| item.name == "just")
        .expect("the table carries just")
        .recipe
        .at(platform)
        .spelled();
    assert!(
        shown.contains(&command),
        "the command was not shown: {shown}"
    );
    assert!(
        machine.asked.lock().unwrap().is_empty(),
        "a no installed something"
    );

    let machine = ScriptedMachine::missing(&["just", "git"]);
    // The table asks about git before just: it is in install order. On
    // Linux git's recipe only prints, because it needs `sudo`, so just is
    // the one question asked there.
    let answers: &[u8] = if cfg!(target_os = "linux") {
        b"y\n"
    } else {
        b"n\ny\n"
    };
    let mut agreed = std::io::Cursor::new(answers.to_vec());
    let mut screen: Vec<u8> = Vec::new();
    run(
        &Asked {
            install: true,
            city: None,
            scanned: std::env::temp_dir(),
            explain: None,
            ink: Ink::Plain,
        },
        &machine,
        &mut agreed,
        &mut screen,
    )
    .unwrap();
    assert_eq!(
        machine.asked.lock().unwrap().as_slice(),
        ["just".to_owned()],
        "exactly the item that was answered yes is installed"
    );
}

/// Checking is what the default does, and it changes nothing.
#[test]
fn the_default_checks_and_installs_nothing() {
    let machine =
        ScriptedMachine::missing(&["just", "gecko", "webkit", "chromedriver", "msedgedriver"]);
    let mut nobody = std::io::Cursor::new(Vec::new());
    let mut screen: Vec<u8> = Vec::new();
    run(
        &Asked {
            install: false,
            city: None,
            scanned: std::env::temp_dir(),
            explain: None,
            ink: Ink::Plain,
        },
        &machine,
        &mut nobody,
        &mut screen,
    )
    .unwrap();
    assert!(machine.asked.lock().unwrap().is_empty());
    let shown = String::from_utf8(screen).unwrap();
    assert!(
        shown.contains("not ready to use: missing a browser engine"),
        "{shown}"
    );
    assert!(
        !shown.contains("[y/N]"),
        "the default asks nothing: {shown}"
    );
}

/// A name the requirement table does not carry is refused, and the
/// refusal says where a working name comes from: the items this machine
/// answered with.
#[test]
fn a_name_the_table_does_not_carry_says_where_a_working_name_comes_from() {
    let refused = crate::doctor::recipe_for("curl | sh")
        .map(|_recipe| ())
        .unwrap_err();
    assert_eq!(
        (
            *refused.code(),
            refused
                .recovery()
                .contains("install one of the items it answered with")
        ),
        (kernel::AxCode::InvalidArgs, true),
        "the refusal says where a working name comes from: {}",
        refused.recovery()
    );
}
