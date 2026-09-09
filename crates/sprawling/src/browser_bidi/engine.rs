// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which engine this machine can hold a session with, and what starting
//! it looks like on a command line.
//!
//! Firefox is the first engine because it speaks the protocol itself: a
//! machine with Firefox on it is a machine this tool works on, with
//! nothing downloaded. Chromium is reachable only through
//! `chromedriver`, so it is a second road rather than a second lane —
//! when the driver is absent this says so by name instead of falling
//! back quietly to something the person did not ask for.
//!
//! Which programs exist on this machine is not decided here. The caller
//! passes what it found, because `bin::doctor` already owns that
//! question and a second answer to it would drift.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use kernel::{AxCode, AxError};

/// The engine a session will be held with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Engine {
    /// Native BiDi: the browser is the server, and no driver stands
    /// between them.
    Firefox { program: PathBuf },
    /// BiDi through a driver process, which is the only way in.
    Chromium { driver: PathBuf },
}

/// What one launch runs, and where it will answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchPlan {
    pub(crate) program: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) port: u16,
}

impl LaunchPlan {
    /// Where the frames go once the process is up.
    pub(crate) fn socket_url(&self) -> String {
        format!("ws://127.0.0.1:{}/session", self.port)
    }
}

impl Engine {
    /// Picks the engine this machine can hold a session with.
    ///
    /// # Errors
    /// Refuses a machine with neither, naming both roads: installing
    /// Firefox is one step and needs no driver, which is why it is
    /// stated first.
    pub(crate) fn choose(
        firefox: Option<&Path>,
        chromedriver: Option<&Path>,
    ) -> Result<Engine, AxError> {
        if let Some(program) = firefox {
            return Ok(Engine::Firefox {
                program: program.to_path_buf(),
            });
        }
        if let Some(driver) = chromedriver {
            return Ok(Engine::Chromium {
                driver: driver.to_path_buf(),
            });
        }
        Err(AxError::failure(
            AxCode::BrowserUnavailable,
            "start a browser",
            "neither firefox nor chromedriver is on this machine",
        )
        .with_recovery(
            "install Firefox, which speaks this protocol itself; `sprawling doctor` installs it",
        ))
    }

    /// The command line, spelled out.
    ///
    /// The port arrives as a parameter rather than being drawn here: two
    /// cities on one machine each open their own browser, and a fixed
    /// port would quietly attach the second to the first one's session.
    /// The profile is the building's own, so what a browser remembers
    /// belongs to a building rather than to the machine.
    pub(crate) fn plan(&self, profile: &Path, port: u16, headless: bool) -> LaunchPlan {
        match self {
            Engine::Firefox { program } => {
                let mut args = vec![
                    "--remote-debugging-port".to_owned(),
                    port.to_string(),
                    "-profile".to_owned(),
                    profile.display().to_string(),
                    "--no-remote".to_owned(),
                ];
                if headless {
                    args.push("-headless".to_owned());
                }
                LaunchPlan {
                    program: program.clone(),
                    args,
                    port,
                }
            }
            Engine::Chromium { driver } => LaunchPlan {
                program: driver.clone(),
                args: vec![format!("--port={port}")],
                port,
            },
        }
    }

    /// Starts the process this plan describes.
    ///
    /// # Errors
    /// Reports a program that will not start, which on this path means
    /// the browser was found and then could not be run — a different
    /// fact from not having one, and the recovery says so.
    pub(crate) fn launch(plan: &LaunchPlan) -> Result<Engaged, AxError> {
        let child = Command::new(&plan.program)
            .args(&plan.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| {
                AxError::failure(
                    AxCode::BrowserUnavailable,
                    "start a browser",
                    format!("{}: {err}", plan.program.display()),
                )
                .with_recovery("check that the program is executable, and that the profile exists")
            })?;
        Ok(Engaged { child })
    }
}

/// A running engine, stopped when this value is dropped.
///
/// Dropping is the whole lifetime rule: a city that ends leaves no
/// browser behind, and one that crashes leaves the operating system to
/// collect it.
pub(crate) struct Engaged {
    child: Child,
}

impl Drop for Engaged {
    fn drop(&mut self) {
        // A browser that has already exited is the outcome this asks
        // for, so its refusal is not news.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The port an engine started for this city will answer on.
///
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

    fn firefox() -> Engine {
        Engine::choose(Some(Path::new("/usr/bin/firefox")), None).unwrap()
    }

    #[test]
    fn firefox_is_the_first_engine_and_needs_no_driver() {
        assert_eq!(
            Engine::choose(
                Some(Path::new("/usr/bin/firefox")),
                Some(Path::new("/usr/bin/chromedriver")),
            )
            .unwrap(),
            Engine::Firefox {
                program: PathBuf::from("/usr/bin/firefox")
            }
        );
    }

    #[test]
    fn chromium_is_reachable_only_through_its_driver() {
        let engine = Engine::choose(None, Some(Path::new("/usr/bin/chromedriver"))).unwrap();
        let plan = engine.plan(Path::new("/city/lab/profile"), 41235, false);
        assert_eq!(plan.args, vec!["--port=41235".to_owned()]);
        let err = Engine::choose(None, None).unwrap_err();
        assert_eq!(err.code(), &AxCode::BrowserUnavailable);
        assert!(err.recovery().contains("Firefox"));
    }

    #[test]
    fn a_launch_names_the_port_the_profile_and_nothing_it_was_not_asked_for() {
        let plan = firefox().plan(Path::new("/city/lab/profile"), 41235, false);
        assert_eq!(
            plan.args,
            vec![
                "--remote-debugging-port".to_owned(),
                "41235".to_owned(),
                "-profile".to_owned(),
                "/city/lab/profile".to_owned(),
                "--no-remote".to_owned(),
            ]
        );
        assert!(
            !plan.args.iter().any(|arg| arg == "-headless"),
            "a window a person can watch is the default"
        );
        assert_eq!(plan.socket_url(), "ws://127.0.0.1:41235/session");
    }

    #[test]
    fn headless_is_asked_for_rather_than_assumed() {
        let plan = firefox().plan(Path::new("/city/lab/profile"), 41235, true);
        assert_eq!(plan.args.last().map(String::as_str), Some("-headless"));
    }

    #[test]
    fn two_cities_on_one_machine_do_not_share_a_port() {
        let one = firefox().plan(Path::new("/a"), 41235, false);
        let two = firefox().plan(Path::new("/b"), 52001, false);
        assert_ne!(one.socket_url(), two.socket_url());
        assert_ne!(one.args, two.args);
    }
}
