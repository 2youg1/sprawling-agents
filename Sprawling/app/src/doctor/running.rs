// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::process::{Child, Stdio};
use std::time::Duration;

use accounting::Runnable;
use kernel::{AxCode, AxError};

pub(crate) const PATIENCE: u32 = 3_600;

const BUILD_PATIENCE: u32 = 24_000;

pub(crate) fn patience_for(runnable: &Runnable) -> u32 {
    match runnable.program() {
        "cargo" => BUILD_PATIENCE,
        _ => PATIENCE,
    }
}

const TICK: Duration = Duration::from_millis(50);

pub(crate) fn run(
    item: &str,
    runnable: &Runnable,
    patience: u32,
    log: &Path,
) -> Result<(), AxError> {
    let (stdout, stderr) = log_streams(item, log)?;
    let mut child = child::command(runnable.program())
        .args(runnable.args())
        .env("PATH", super::host::search_path())
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|err| {
            AxError::failure(
                AxCode::ToolUnavailable,
                "install a tool",
                format!("{item}: {}: {err}", runnable.program()),
            )
            .with_recovery("install the package manager first, or run the printed line yourself")
        })?;
    let mut knocks = patience;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_ended_in_failure)) => {
                return Err(AxError::failure(
                    AxCode::ToolUnavailable,
                    "install a tool",
                    format!("{item}: {} ended in failure", runnable.spelled()),
                )
                .with_recovery(format!(
                    "read what it reported in {}, then run the line yourself in a terminal",
                    log.display()
                )));
            }
            Ok(None) => {}
            Err(err) => {
                return Err(unwatchable(item, runnable, &stop(&mut child), &err));
            }
        }
        let Some(left) = knocks.checked_sub(1) else {
            let waited = Waited { patience, log };
            return Err(overran(item, runnable, &waited, &stop(&mut child)));
        };
        knocks = left;
        std::thread::sleep(TICK);
    }
}

fn log_streams(item: &str, log: &Path) -> Result<(Stdio, Stdio), AxError> {
    let refused = |err: std::io::Error| {
        AxError::failure(
            AxCode::ToolUnavailable,
            "install a tool",
            format!(
                "{item}: its log {} could not be written: {err}",
                log.display()
            ),
        )
        .with_recovery("free the temporary directory, or run the line yourself in a terminal")
    };
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).map_err(refused)?;
    }
    let file = std::fs::File::create(log).map_err(refused)?;
    let second = file.try_clone().map_err(refused)?;
    Ok((Stdio::from(file), Stdio::from(second)))
}

pub(super) fn stop(child: &mut Child) -> Option<String> {
    if let Err(err) = child.kill() {
        return Some(format!("it could not be stopped: {err}"));
    }
    match child.wait() {
        Ok(_reaped) => None,
        Err(err) => Some(format!("it was stopped but not reaped: {err}")),
    }
}

struct Waited<'a> {
    patience: u32,
    log: &'a Path,
}

fn overran(item: &str, runnable: &Runnable, waited: &Waited, stopping: &Option<String>) -> AxError {
    let seconds = TICK.saturating_mul(waited.patience).as_secs();
    let aftermath = match stopping {
        None => "it was stopped".to_owned(),
        Some(trouble) => trouble.clone(),
    };
    AxError::failure(
        AxCode::Timeout,
        "install a tool",
        format!(
            "{item}: {} was still running after {seconds}s, so {aftermath}",
            runnable.spelled()
        ),
    )
    .with_recovery(format!(
        "run this line yourself in a terminal, where it can take as long as it needs and ask \
         what it wants to ask; what it printed before it was stopped is in {}",
        waited.log.display()
    ))
}

fn unwatchable(
    item: &str,
    runnable: &Runnable,
    stopping: &Option<String>,
    err: &std::io::Error,
) -> AxError {
    let aftermath = match stopping {
        None => "it was stopped".to_owned(),
        Some(trouble) => trouble.clone(),
    };
    AxError::failure(
        AxCode::ToolOutcomeUnknown,
        "install a tool",
        format!(
            "{item}: {} could not be waited for: {err}; {aftermath}",
            runnable.spelled()
        ),
    )
    .with_recovery("check whether the tool is installed, and run the line yourself if it is not")
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

    use accounting::Recipe;

    fn ending(code: Code) -> Recipe {
        match (cfg!(target_os = "windows"), code) {
            (true, Code::Success) => Recipe::Command {
                program: "cmd",
                args: &["/C", "exit", "0"],
            },
            (true, Code::Failure) => Recipe::Command {
                program: "cmd",
                args: &["/C", "exit", "3"],
            },
            (false, Code::Success) => Recipe::Command {
                program: "sh",
                args: &["-c", "exit 0"],
            },
            (false, Code::Failure) => Recipe::Command {
                program: "sh",
                args: &["-c", "exit 3"],
            },
        }
    }

    fn scratch_log(test: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("sprawling-running-{test}.log"))
    }

    enum Code {
        Success,
        Failure,
    }

    fn never_ending() -> Recipe {
        if cfg!(target_os = "windows") {
            Recipe::Command {
                program: "ping",
                args: &["-n", "30", "127.0.0.1"],
            }
        } else {
            Recipe::Command {
                program: "sleep",
                args: &["30"],
            }
        }
    }

    fn waiting_on_input() -> Recipe {
        if cfg!(target_os = "windows") {
            Recipe::Command {
                program: "cmd",
                args: &["/C", "pause"],
            }
        } else {
            Recipe::Command {
                program: "sh",
                args: &["-c", "read answer"],
            }
        }
    }

    #[test]
    fn a_program_that_never_ends_is_killed_when_the_knocks_run_out() {
        let recipe = never_ending();
        let runnable = recipe.command("pretend").unwrap();

        let refused = run("pretend", &runnable, 40, &scratch_log("never_ends")).unwrap_err();
        assert_eq!(refused.code(), &AxCode::Timeout);
        assert!(
            refused.recovery().contains("run this line yourself"),
            "the refusal says what works instead: {}",
            refused.recovery()
        );
    }

    #[test]
    fn a_program_that_asks_a_question_ends_long_before_the_knocks_do() {
        let recipe = waiting_on_input();
        let runnable = recipe.command("pretend").unwrap();

        let outcome = run("pretend", &runnable, 100, &scratch_log("asks"));
        assert!(
            outcome.as_ref().err().map(AxError::code) != Some(&AxCode::Timeout),
            "it read end of input rather than waiting for a person: {outcome:?}"
        );
    }

    #[test]
    fn a_program_that_succeeds_is_reported_as_done() {
        let recipe = ending(Code::Success);
        assert!(
            run(
                "pretend",
                &recipe.command("pretend").unwrap(),
                PATIENCE,
                &scratch_log("succeeds")
            )
            .is_ok()
        );
    }

    #[test]
    fn a_program_that_fails_is_reported_with_the_line_a_person_can_rerun() {
        let recipe = ending(Code::Failure);
        let refused = run(
            "pretend",
            &recipe.command("pretend").unwrap(),
            PATIENCE,
            &scratch_log("fails"),
        )
        .unwrap_err();
        assert_eq!(refused.code(), &AxCode::ToolUnavailable);
        assert!(refused.to_string().contains("ended in failure"));
    }

    #[test]
    fn a_program_that_is_not_on_this_machine_is_refused_before_any_wait() {
        let recipe = Recipe::Command {
            program: "sprawling-no-such-installer",
            args: &[],
        };
        let runnable = recipe.command("pretend").unwrap();
        let refused = run("pretend", &runnable, PATIENCE, &scratch_log("absent")).unwrap_err();
        assert_eq!(refused.code(), &AxCode::ToolUnavailable);
    }

    #[test]
    fn what_an_installer_prints_lands_in_its_log() {
        let recipe = if cfg!(target_os = "windows") {
            Recipe::Command {
                program: "cmd",
                args: &["/C", "echo fetched & echo refused 1>&2 & exit 3"],
            }
        } else {
            Recipe::Command {
                program: "sh",
                args: &["-c", "echo fetched; echo refused >&2; exit 3"],
            }
        };
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("pretend.log");
        let refused = run(
            "pretend",
            &recipe.command("pretend").unwrap(),
            PATIENCE,
            &log,
        )
        .unwrap_err();
        let written = std::fs::read_to_string(&log).unwrap();
        assert_eq!(
            (
                written.contains("fetched"),
                written.contains("refused"),
                refused.recovery().contains(&log.display().to_string())
            ),
            (true, true, true),
            "the log holds both streams and the refusal names it: {written:?} / {}",
            refused.recovery()
        );
    }

    #[test]
    fn a_build_is_given_twenty_minutes_and_a_download_three() {
        let cargo = Recipe::Command {
            program: "cargo",
            args: &["install", "cargo-deny", "--locked"],
        };
        let winget = Recipe::Command {
            program: "winget",
            args: &["install", "--id", "Casey.Just"],
        };
        let waits = [cargo, winget]
            .map(|recipe| TICK.saturating_mul(patience_for(&recipe.command("pretend").unwrap())));
        assert_eq!(
            waits,
            [Duration::from_secs(20 * 60), Duration::from_secs(180)]
        );
    }
}
