// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How a refusal is written on the command line (sprawling-SPEC.md 8-91):
//! the text alone, so the caller decides where it goes.

use kernel::AxError;

/// Who reads the refusal: a person, or a program that deserializes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Form {
    Human,
    Json,
}

impl Form {
    /// `--json` anywhere on the line asks for the machine form.
    pub(super) fn of(args: &[String]) -> Self {
        if args.iter().any(|a| a == "--json") {
            Form::Json
        } else {
            Form::Human
        }
    }
}

/// The refusal's text, ending in a newline. The json form is one line;
/// should the error ever fail to serialize, the person's form is written
/// with the reason, so the refusal itself is never lost.
pub(super) fn written(err: &AxError, form: Form) -> String {
    match form {
        Form::Human => human(err),
        Form::Json => match serde_json::to_string(err) {
            Ok(line) => format!("{line}\n"),
            Err(fail) => format!(
                "{}could not write this refusal as json: {fail}\n",
                human(err)
            ),
        },
    }
}

fn human(err: &AxError) -> String {
    let recovery = format!("{err}\nrecovery: {}\n", err.recovery());
    match err.nearby() {
        [] => recovery,
        nearby => format!("{recovery}nearby: {}\n", nearby.join(", ")),
    }
}
