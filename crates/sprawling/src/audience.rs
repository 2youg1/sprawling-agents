// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Who reads stdout, decided in this one place (sprawling-SPEC.md
//! 8-129-2): a person at a terminal, or an agent reading a pipe or a
//! file. `view`, `gauge` and `gauge --at` all ask here, so a line meant
//! for an agent cannot reach a terminal by one verb's rule and not by
//! another's.

use std::io::IsTerminal;

/// Who reads what a command writes to stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    /// A pipe or a file: one machine-readable line per reading.
    Agent,
    /// A terminal: lines in units, or a redrawn screen.
    Person,
}

impl Audience {
    /// Who reads this process's stdout.
    #[must_use]
    pub fn of_stdout() -> Audience {
        Self::of(&std::io::stdout())
    }

    /// A terminal is read by a person; anything else by an agent.
    fn of(stream: &impl IsTerminal) -> Audience {
        let _terminal = stream.is_terminal();
        Audience::Person
    }
}

#[cfg(test)]
#[path = "audience/tests.rs"]
mod tests;
