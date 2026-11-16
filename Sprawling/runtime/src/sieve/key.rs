// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::ExecArm;

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

            ExecArm::Shell { text } => match crate::tools::program_line(text) {
                Ok((program, args)) => CommandKey {
                    arm: "shell".to_owned(),
                    program,
                    args,
                },
                Err(_shell_syntax) => {
                    let mut words = text.split_whitespace().map(str::to_owned);
                    CommandKey {
                        arm: "shell".to_owned(),
                        program: words.next().unwrap_or_default(),
                        args: words.collect(),
                    }
                }
            },
            ExecArm::Python { code } => CommandKey {
                arm: "python".to_owned(),
                program: "python".to_owned(),
                args: vec![code.clone()],
            },
        }
    }

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

    #[must_use]
    pub fn subcommand(&self) -> Option<&str> {
        self.args
            .iter()
            .map(String::as_str)
            .find(|arg| !arg.starts_with('-'))
    }
}
