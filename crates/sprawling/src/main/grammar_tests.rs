// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::panic)]

use super::super::verbs::{Effect, VERBS, Verb};
use super::{Invocation, LineError, parse};

fn words(line: &[&str]) -> Vec<String> {
    line.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn help_wins_over_every_verb_that_changes_something() {
    for row in VERBS.iter().filter(|row| row.effect == Effect::Changes) {
        for asked in ["--help", "-h"] {
            assert_eq!(
                parse(&words(&[row.name, asked])),
                Ok(Invocation::Help(row.verb)),
                "{} {asked} would run the verb",
                row.name
            );
        }
    }
    assert_eq!(
        parse(&words(&["install", "--help"])),
        Ok(Invocation::Help(Verb::Install))
    );
    assert_eq!(
        parse(&words(&["up", "--help"])),
        Ok(Invocation::Help(Verb::Up))
    );
}

#[test]
fn a_flag_value_is_not_an_address() {
    let Ok(Invocation::Run(Verb::Serve, read)) =
        parse(&words(&["serve", "city", "--log", "debug"]))
    else {
        panic!("serve city --log debug did not parse as serve");
    };
    assert_eq!(read.value("--log"), Some("debug"));
    assert_eq!(read.positional(1).map(String::as_str), Some("city"));
    assert_eq!(read.positional(2), None, "the address took the log level");
}

/// `-m` is the short spelling of `--model`, so a body reads one name
/// whichever the person typed.
#[test]
fn a_short_flag_reads_as_its_long_name() {
    for spelled in ["-m", "--model"] {
        let line = ["dispatch", "lab/room1", "write it", spelled, "m-other"];
        let Ok(Invocation::Run(Verb::Dispatch, read)) = parse(&words(&line)) else {
            panic!("{line:?} did not parse as dispatch");
        };
        assert_eq!(read.value("--model"), Some("m-other"), "{line:?}");
        assert_eq!(
            read.positional(3),
            None,
            "{line:?}: the model id is no positional"
        );
    }
}

#[test]
fn a_mistyped_verb_names_the_nearest() {
    assert_eq!(
        parse(&words(&["stauts"])),
        Err(LineError::UnknownVerb {
            given: "stauts".to_owned(),
            nearest: vec!["status"],
        })
    );
}

#[test]
fn an_unknown_flag_is_refused() {
    assert!(matches!(
        parse(&words(&["replay", "dir", "--jsno"])),
        Err(LineError::UnknownFlag { verb: "replay", .. })
    ));
}

/// Every flag a verb body reads from the raw line (`city.rs` `serve_city`,
/// which `up` reaches too, and `doctor/screen.rs`) must pass the table, or
/// the parser refuses a flag that works.
#[test]
fn every_flag_a_body_reads_passes_the_table() {
    let served: &[&[&str]] = &[
        &["--open"],
        &["--no-open"],
        &["--console"],
        &["--no-console"],
        &["--log", "debug"],
        &["--web-dir", "dir"],
    ];
    for verb in ["up", "serve"] {
        for flag in served {
            let line = [&[verb, "city"][..], flag].concat();
            assert!(
                matches!(parse(&words(&line)), Ok(Invocation::Run(..))),
                "{line:?} is refused"
            );
        }
    }
    for flag in [&["--install"][..], &["--explain", "E1"], &["--no-color"]] {
        let line = [&["doctor"][..], flag].concat();
        assert!(
            matches!(parse(&words(&line)), Ok(Invocation::Run(..))),
            "{line:?} is refused"
        );
    }
}
