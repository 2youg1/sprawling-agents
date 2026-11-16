// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;
use std::time::Duration;

use crate::report::XtaskError;

const PROFILE_RELEASE: Duration = Duration::from_secs(5);

pub(super) const LOGS: &str = "engine";

#[derive(Clone, Copy)]
pub(super) enum Attempt {
    First,
    Retry,
}

impl Attempt {
    pub(super) fn log_suffix(self) -> &'static str {
        match self {
            Attempt::First => ".log",
            Attempt::Retry => ".retry.log",
        }
    }
}

#[derive(Debug)]
pub(super) enum Took {
    First,

    Retried(String),
}

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
