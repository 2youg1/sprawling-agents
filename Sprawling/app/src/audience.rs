// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {

    Agent,

    Person,
}

impl Audience {

    #[must_use]
    pub fn of_stdout() -> Audience {
        Self::of(&std::io::stdout())
    }

    fn of(stream: &impl IsTerminal) -> Audience {
        if stream.is_terminal() {
            Audience::Person
        } else {
            Audience::Agent
        }
    }
}

#[cfg(test)]
#[path = "audience/tests.rs"]
mod tests;
