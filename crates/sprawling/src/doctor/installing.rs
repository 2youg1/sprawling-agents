// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Getting one thing this machine lacks, asked for from a page rather
//! than from a terminal (sprawling-SPEC.md section 8-40).
//!
//! **The rule the terminal follows is the rule here.** `doctor::screen`
//! offers a person every absent item one at a time and runs only a
//! `Recipe::Command`; this runs the same recipe through the same
//! `Machine::install`, so a script piped into a shell stays code nobody
//! read whichever door the request arrived at. What is added here is
//! the naming: a page sends a name, and a name that is not in the
//! requirement table is refused rather than searched for.
//!
//! Progress is log lines. There is no second progress channel, because
//! an install is one process this city starts and waits for, and the
//! two facts worth reporting - what is about to run, and how it ended -
//! are exactly what a log line is for.

use kernel::{AxCode, AxError};

use super::{Machine, PATIENCE, Platform, REQUIREMENTS, Recipe, Requirement, ThisMachine};

/// Runs the recipe this platform has for one named requirement.
///
/// `progress` is told what is about to run and how it ended. It is a
/// parameter rather than a field because the caller is the only thing
/// that knows where a person is reading.
///
/// # Errors
/// Refuses a name the requirement table does not carry, a platform this
/// project states no recipes for, a recipe this city may not run, and
/// whatever starting the program reports.
pub(crate) fn install(item: &str, progress: &mut dyn FnMut(&str)) -> Result<(), AxError> {
    let requirement = named(item)?;
    let Some(platform) = Platform::current() else {
        return Err(AxError::failure(
            AxCode::ToolUnavailable,
            "install a tool",
            format!("{item}: this platform has no recipe in this project"),
        )
        .with_recovery("install it the way this operating system installs software"));
    };
    let recipe = requirement.recipe.at(platform);
    if !recipe.runnable() {
        return Err(refuse_to_run(item, recipe));
    }
    progress(&format!("installing {item}: {}", recipe.spelled()));
    ThisMachine::new(Some(platform), PATIENCE).install(item, recipe)?;
    progress(&format!("installed {item}; this city looked again"));
    Ok(())
}

/// The row of the requirement table this name belongs to.
///
/// A name the table does not carry is refused with the fact that
/// settles it: the page is asking about an item this build does not
/// know, so re-reading what the city answered is the way out.
fn named(item: &str) -> Result<&'static Requirement, AxError> {
    REQUIREMENTS
        .iter()
        .find(|requirement| requirement.name == item)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "install a tool",
                format!("{item}: this city checks for no such item"),
            )
            .with_recovery("ask this machine again and install one of the items it answered with")
        })
}

/// Why a recipe that is not a command is not run, in the words that say
/// what the person does instead.
fn refuse_to_run(item: &str, recipe: &Recipe) -> AxError {
    let recovery = match recipe {
        // Unreachable by the caller's guard, and answered rather than
        // asserted: a command is what `runnable` means.
        Recipe::Command { .. } => "run it from here",
        Recipe::Print(_) => {
            "run the printed line yourself: a script piped into a shell is code nobody read, and \
             this city does not read it for you"
        }
        Recipe::Manual(_) => "follow the printed instruction: nothing here can install this one",
    };
    AxError::failure(
        AxCode::ToolUnavailable,
        "install a tool",
        format!("{item}: {}", recipe.spelled()),
    )
    .with_recovery(recovery)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A name nobody offered is refused before any process starts, and
    /// the refusal says how to find a name that works.
    #[test]
    fn an_item_the_table_does_not_carry_starts_nothing() {
        let mut said: Vec<String> = Vec::new();
        let refused = install("curl | sh", &mut |line| said.push(line.to_owned())).unwrap_err();
        assert_eq!(refused.code(), &AxCode::InvalidArgs);
        assert!(said.is_empty(), "nothing was reported as running: {said:?}");
    }

    /// The two recipes this city may not run say what the person does
    /// instead, and neither of them says it the same way.
    #[test]
    fn a_printed_recipe_and_a_manual_one_refuse_with_their_own_reason() {
        let printed = refuse_to_run("bun", &Recipe::Print("curl -fsSL https://bun.sh/install"));
        assert_eq!(printed.code(), &AxCode::ToolUnavailable);
        assert!(printed.recovery().contains("code nobody read"));
        let by_hand = refuse_to_run("shell", &Recipe::Manual("it ships with the system"));
        assert!(by_hand.recovery().contains("nothing here can install"));
        assert_ne!(printed.recovery(), by_hand.recovery());
    }
}
