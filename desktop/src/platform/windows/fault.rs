// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one place a Win32 failure becomes a refusal a caller can act on.
//!
//! Every module under this one funnels here, for a reason worth stating:
//! an operating-system error code is a fact about this machine, and a
//! caller on the other end of a pipe can do nothing with the number
//! itself. What it can do something with is the sentence beside it, so
//! each call site supplies **what it was doing** and **what to try
//! instead**, and this module supplies the machine's own words for why
//! it did not work.
//!
//! `E_TOOL_UNAVAILABLE` rather than `E_INVALID_ARGS` throughout: the
//! caller's arguments were already judged — by the scope file, then by
//! `target` and `views` — so a failure that gets this far is this
//! machine's, and telling a model to fix its arguments would send it
//! rewriting a call that was correct.

use desktop_ffi::ended::Failure;
use desktop_ffi::step::Step;

use crate::refusal::{Refusal, RefusalCode};

/// What the operating system said through `winsafe` or through the Zig
/// leaf, as a refusal.
///
/// `doing` names the operation in a caller's vocabulary rather than the
/// API's — "read the window's title", not "GetWindowTextW". Which
/// binding carried the call is this server's business, and the caller
/// reads the machine's words.
pub(crate) fn system(doing: &str, recovery: &str, err: winsafe::co::ERROR) -> Refusal {
    refused(doing, recovery, &err.to_string())
}

/// What COM, or shcore, said in an HRESULT.
pub(crate) fn com(doing: &str, recovery: &str, err: winsafe::co::HRESULT) -> Refusal {
    refused(doing, recovery, &err.to_string())
}

/// What UI Automation said, as a refusal.
pub(crate) fn automation(doing: &str, recovery: &str, err: &uiautomation::Error) -> Refusal {
    refused(doing, recovery, err.message())
}

/// A failure of the leaf whose step this operation has but whose
/// meaning is the step's alone: the machine's reason, in the machine's
/// words, for the one thing `doing` names.
pub(crate) fn leaf(doing: &str, recovery: &str, failure: Failure) -> Refusal {
    match failure {
        Failure::At { code, .. } => system(doing, recovery, code),
        Failure::Unspelled(raw) => unspelled(doing, raw),
    }
}

/// The leaf answered a step the operation `doing` does not have. The
/// leaf and this server are built together, so this is a defect in this
/// server, and it is reported as one rather than dressed as the
/// machine's refusal.
pub(crate) fn stray(doing: &str, step: Step) -> Refusal {
    defect(
        doing,
        &format!("its leaf stopped at {step:?}, a step this operation does not have"),
    )
}

/// The leaf answered a number no step has: a leaf and a server from two
/// builds.
pub(crate) fn unspelled(doing: &str, raw: u32) -> Refusal {
    defect(
        doing,
        &format!("its leaf answered step {raw}, which this build does not spell"),
    )
}

fn defect(doing: &str, said: &str) -> Refusal {
    Refusal::new(
        RefusalCode::ToolUnavailable,
        "use the desktop",
        format!("this server could not {doing}: {said}"),
        "this is a defect in this server rather than in the call; report it",
    )
}

/// The one sentence every machine failure is told in.
fn refused(doing: &str, recovery: &str, said: &str) -> Refusal {
    Refusal::new(
        RefusalCode::ToolUnavailable,
        "use the desktop",
        format!("this machine refused to {doing}: {said}"),
        recovery,
    )
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

    /// A machine failure is this machine's, and the refusal says so
    /// rather than telling a model its correct call was wrong.
    #[test]
    fn a_machine_failure_is_unavailable_rather_than_bad_arguments() {
        let refusal = system(
            "read the window's title",
            "call `desktop.windows` again",
            winsafe::co::ERROR::ACCESS_DENIED,
        );
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert_eq!(error["code"], -32000);
        assert!(
            error["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("read the window's title")
        );
        assert_eq!(error["data"]["recovery"], "call `desktop.windows` again");
    }

    /// A step the leaf should not have answered, and a number that is no
    /// step, still arrive with three parts, and say they are this
    /// server's defect.
    #[test]
    fn a_step_this_operation_does_not_have_still_carries_three_parts() {
        for refusal in [
            stray("capture a window", Step::Handing),
            leaf("capture a window", "try again", Failure::Unspelled(77)),
        ] {
            let error = refusal.as_error();
            assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
            for part in ["action", "subject", "recovery"] {
                assert!(
                    !error["data"][part].as_str().unwrap_or_default().is_empty(),
                    "{part} is empty"
                );
            }
            assert!(
                error["data"]["recovery"]
                    .as_str()
                    .unwrap()
                    .contains("defect")
            );
        }
    }
}
