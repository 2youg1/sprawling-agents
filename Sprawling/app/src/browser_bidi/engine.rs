// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};

use kernel::{AxCode, AxError};

use crate::doctor::Presence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Engine {

    Firefox { program: PathBuf },

    Chromium { driver: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchPlan {
    pub(crate) program: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) port: u16,
}

impl LaunchPlan {

    pub(crate) fn socket_url(&self) -> String {
        format!("ws://127.0.0.1:{}/session", self.port)
    }
}

impl Engine {

    pub(crate) fn choose(firefox: &Presence, chromedriver: &Presence) -> Result<Engine, AxError> {
        if let Presence::Present { at, .. } = firefox {
            return Ok(Engine::Firefox {
                program: at.clone(),
            });
        }
        if let Presence::Present { at, .. } = chromedriver {
            return Ok(Engine::Chromium { driver: at.clone() });
        }
        let recovery = if matches!(firefox, Presence::Broken { .. }) {
            "run Firefox by hand once to see what it reports; `sprawling doctor` names the fault"
        } else {
            "install Firefox, which speaks this protocol itself; `sprawling doctor --install` offers it"
        };
        Err(AxError::failure(
            AxCode::BrowserUnavailable,
            "start a browser",
            format!(
                "firefox {}; chromedriver {}",
                firefox.describe(),
                chromedriver.describe()
            ),
        )
        .with_recovery(recovery))
    }

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

    pub(crate) fn launch(plan: &LaunchPlan) -> Result<Engaged, AxError> {
        let child = child::command(&plan.program)
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

pub(crate) struct Engaged {
    child: Child,
}

impl Drop for Engaged {
    fn drop(&mut self) {

        drop(self.child.kill());
        drop(self.child.wait());
    }
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

    fn present(at: &str) -> Presence {
        Presence::Present {
            at: PathBuf::from(at),
            version: crate::doctor::Version::Silent,
        }
    }

    fn absent() -> Presence {
        Presence::Absent(crate::doctor::Absence::NotOnSearchPath)
    }

    fn firefox() -> Engine {
        Engine::choose(&present("/usr/bin/firefox"), &absent()).unwrap()
    }

    #[test]
    fn firefox_is_the_first_engine_and_needs_no_driver() {
        assert_eq!(
            Engine::choose(
                &present("/usr/bin/firefox"),
                &present("/usr/bin/chromedriver"),
            )
            .unwrap(),
            Engine::Firefox {
                program: PathBuf::from("/usr/bin/firefox")
            }
        );
    }

    #[test]
    fn chromium_is_reachable_only_through_its_driver() {
        let engine = Engine::choose(&absent(), &present("/usr/bin/chromedriver")).unwrap();
        let plan = engine.plan(Path::new("/city/lab/profile"), 41235, false);
        assert_eq!(plan.args, vec!["--port=41235".to_owned()]);
        let err = Engine::choose(&absent(), &absent()).unwrap_err();
        assert_eq!(err.code(), &AxCode::BrowserUnavailable);
        assert!(err.recovery().contains("Firefox"));
    }

    #[test]
    fn a_broken_firefox_is_refused_by_its_fault_not_as_absent() {
        let broken = Presence::Broken {
            at: PathBuf::from("/usr/bin/firefox"),
            fault: crate::doctor::Fault::WillNotStart("permission denied".to_owned()),
        };
        let err = Engine::choose(&broken, &absent()).unwrap_err();
        assert_eq!(err.code(), &AxCode::BrowserUnavailable);
        let said = err.to_string();
        assert!(said.contains("permission denied"), "{said}");
        assert!(said.contains("not on the search path"), "{said}");
        assert!(
            !err.recovery().contains("install Firefox"),
            "{}",
            err.recovery()
        );
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
