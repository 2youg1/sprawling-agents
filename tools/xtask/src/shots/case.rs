// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One case of `shots` - one measuring or one picture - as tries: the
//! second try a failed first one gets, the failure named for the case,
//! and the profile directory each try removes after itself
//! (tools/xtask/Spec.lean §8-44).

use std::path::Path;
use std::time::Duration;

use crate::report::XtaskError;

/// How long a profile directory is given to come free for removal.
const PROFILE_RELEASE: Duration = Duration::from_secs(5);

/// The directory under the run's output that holds each case's stderr.
pub(super) const LOGS: &str = "engine";

/// Which try of a case a launch is.
#[derive(Clone, Copy)]
pub(super) enum Attempt {
    First,
    Retry,
}

impl Attempt {
    /// What the stderr file of this try ends in.
    pub(super) fn log_suffix(self) -> &'static str {
        match self {
            Attempt::First => ".log",
            Attempt::Retry => ".retry.log",
        }
    }
}

/// How a case came to succeed.
#[derive(Debug)]
pub(super) enum Took {
    First,
    /// Its first try failed for this reason, and the second succeeded.
    Retried(String),
}

/// Runs `attempt` once, and once more when it fails; the second
/// failure is the case's, named with both reasons.
pub(super) fn twice<T>(
    case: &str,
    mut attempt: impl FnMut(Attempt) -> Result<T, XtaskError>,
) -> Result<(T, Took), XtaskError> {
    match attempt(Attempt::First) {
        Ok(done) => Ok((done, Took::First)),
        Err(first) => match attempt(Attempt::Retry) {
            Ok(done) => Ok((done, Took::Retried(first.to_string()))),
            Err(again) => Err(XtaskError::Cmd {
                cmd: format!("cargo xtask shots, case {case}"),
                msg: format!(
                    "the case failed twice: first {first}; then {again}; read what the engine \
                     said in target/shots/{LOGS}/{case}.log and {case}.retry.log"
                ),
            }),
        },
    }
}

/// Removes `dir` and everything under it; a directory that is not
/// there is already removed.
pub(super) fn removed(dir: &Path) -> Result<(), XtaskError> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(XtaskError::Io {
            path: dir.display().to_string(),
            source,
        }),
    }
}

/// Removes a launch's profile, giving a process still inside it up to
/// [`PROFILE_RELEASE`] to let go of its files.
pub(super) fn released(profile: &Path) -> Result<(), XtaskError> {
    let pause = Duration::from_millis(250);
    let mut waited = Duration::ZERO;
    loop {
        match removed(profile) {
            Ok(()) => return Ok(()),
            Err(err) if waited >= PROFILE_RELEASE => return Err(err),
            Err(_) => {
                std::thread::sleep(pause);
                waited = waited.saturating_add(pause);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    fn failing(why: &str) -> XtaskError {
        XtaskError::Cmd {
            cmd: "engine".to_owned(),
            msg: why.to_owned(),
        }
    }

    #[test]
    fn a_case_that_fails_once_is_retried_and_says_why() {
        let (done, took) = twice("talk-01-1440-dark", |attempt| match attempt {
            Attempt::First => Err(failing("stalled")),
            Attempt::Retry => Ok(7),
        })
        .unwrap();
        assert_eq!(done, 7);
        assert!(matches!(took, Took::Retried(why) if why.contains("stalled")));
    }

    #[test]
    fn a_case_that_fails_twice_is_named_with_both_reasons() {
        let mut tries = 0;
        let err = twice::<()>("gallery-105-1440-dark", |attempt| {
            tries += 1;
            Err(failing(match attempt {
                Attempt::First => "stalled",
                Attempt::Retry => "stalled again",
            }))
        })
        .unwrap_err()
        .to_string();
        assert_eq!(tries, 2);
        assert!(err.contains("case gallery-105-1440-dark"), "{err}");
        assert!(
            err.contains("first command `engine` failed: stalled;"),
            "{err}"
        );
        assert!(err.contains("stalled again"), "{err}");
        assert!(err.contains("gallery-105-1440-dark.retry.log"), "{err}");
    }
}
