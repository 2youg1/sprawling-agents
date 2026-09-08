// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which command produced an output. The identity a filter matches
//! and the differencing groups by: the arm, the program, its
//! arguments. Ordered so it can key a `BTreeMap`, never a hash.

use kernel::ExecArm;

/// Which command produced an output: the identity a filter matches
/// and the differencing groups by. Ordered so it can key a `BTreeMap`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandKey {
    arm: String,
    program: String,
    args: Vec<String>,
}

impl CommandKey {
    #[must_use]
    pub fn of(arm: &ExecArm) -> CommandKey {
        match arm {
            ExecArm::Program { path, args } => CommandKey {
                arm: "program".to_owned(),
                program: path.clone(),
                args: args.clone(),
            },
            ExecArm::Shell { text } => {
                let mut words = text.split_whitespace().map(str::to_owned);
                CommandKey {
                    arm: "shell".to_owned(),
                    program: words.next().unwrap_or_default(),
                    args: words.collect(),
                }
            }
            ExecArm::Python { code } => CommandKey {
                arm: "python".to_owned(),
                program: "python".to_owned(),
                args: vec![code.clone()],
            },
        }
    }

    /// The program's file name, lower-cased, without `.exe`: what a
    /// filter's `command` is compared to.
    #[must_use]
    pub fn command(&self) -> String {
        let name = self
            .program
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        name.strip_suffix(".exe").unwrap_or(&name).to_owned()
    }

    /// The first argument that is not a flag.
    #[must_use]
    pub fn subcommand(&self) -> Option<&str> {
        self.args
            .iter()
            .map(String::as_str)
            .find(|arg| !arg.starts_with('-'))
    }
}
