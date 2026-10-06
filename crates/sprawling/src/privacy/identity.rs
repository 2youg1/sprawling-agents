// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Read-only principal sampling (`crates/sprawling/spec/Privacy/Cli.lean` D54).

use kernel::{AxCode, AxError};
use zeroize::Zeroizing;

/// The current Windows user's SID, asked of the PowerShell under the
/// protected installation path.
///
/// # Errors
/// `ToolUnavailable` when the installation path, the query or its answer
/// fails; the refusal never repeats the answer.
#[cfg(windows)]
pub(super) fn read() -> Result<Zeroizing<String>, AxError> {
    use super::windows::{PATIENCE, powershell};
    use crate::doctor::asking::{self, Ended};

    let mut command = std::process::Command::new(powershell()?);
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "$ErrorActionPreference = 'Stop'; [Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
    ]);
    let mut first = true;
    match asking::ask(&mut command, PATIENCE, move |_| std::mem::take(&mut first)) {
        Ended::Exited {
            code: Some(0),
            kept,
        } => principal(Zeroizing::new(kept)),
        Ended::Exited {
            code: Some(_) | None,
            ..
        }
        | Ended::Unstarted
        | Ended::Unanswered { .. } => Err(refused("Windows principal query failed")),
    }
}

#[cfg(not(windows))]
pub(super) fn read() -> Result<Zeroizing<String>, AxError> {
    Err(refused(
        "Windows privacy controls are unavailable on this platform",
    ))
}

/// The answer, kept only when it is a SID: `S-1-`, an identifier
/// authority and at least one sub-authority, each a non-empty run of
/// decimal digits.
#[cfg(windows)]
fn principal(answer: Zeroizing<String>) -> Result<Zeroizing<String>, AxError> {
    let decimal =
        |group: &str| !group.is_empty() && group.bytes().all(|byte| byte.is_ascii_digit());
    if answer
        .strip_prefix("S-1-")
        .is_some_and(|groups| groups.split('-').nth(1).is_some() && groups.split('-').all(decimal))
    {
        Ok(answer)
    } else {
        Err(refused(
            "Windows principal query returned no valid identity",
        ))
    }
}

fn refused(subject: &str) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, "read privacy principal", subject).with_recovery(
        "run on Windows with a readable system installation; leave privacy history unchanged",
    )
}

#[cfg(all(test, windows))]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    /// The production path end to end: the protected installation path,
    /// the PowerShell query through `doctor::asking`, and the line it
    /// keeps, which `principal` accepts only as a SID. It only reads, and
    /// a refusal names no answer.
    #[test]
    fn the_running_account_answers_with_a_sid() {
        read().unwrap();
    }

    #[test]
    fn only_a_sid_is_kept_and_a_refusal_never_repeats_the_answer() {
        for sid in ["S-1-5-18", "S-1-5-21-1004336348-1177238915-682003330-1001"] {
            assert_eq!(
                principal(Zeroizing::new(sid.to_owned())).unwrap().as_str(),
                sid
            );
        }
        for answer in [
            "",
            "S-1-",
            "S-1-5",
            "S-1-5-",
            "S-1-5--18",
            "S-1-S-18",
            "S-2-5-18",
        ] {
            let refusal = principal(Zeroizing::new(answer.to_owned())).unwrap_err();
            assert_eq!(
                refusal,
                refused("Windows principal query returned no valid identity")
            );
        }
    }
}
