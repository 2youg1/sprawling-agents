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

use crate::refusal::{Refusal, RefusalCode};

/// What the operating system said, as a refusal.
///
/// `doing` names the operation in a caller's vocabulary rather than the
/// API's — "read the window's title", not "GetWindowTextW".
pub(crate) fn win32(doing: &str, recovery: &str, err: &windows::core::Error) -> Refusal {
    Refusal::new(
        RefusalCode::ToolUnavailable,
        "use the desktop",
        format!("this machine refused to {doing}: {}", err.message()),
        recovery,
    )
}

/// A Win32 call that reports failure without an error code of its own.
///
/// Several of the GDI entry points answer with a null handle or a zero
/// and leave the reason on the calling thread; `Error::from_thread` is
/// what reads it, so those sites still arrive here with a message rather
/// than with silence.
pub(crate) fn last(doing: &str, recovery: &str) -> Refusal {
    win32(doing, recovery, &windows::core::Error::from_thread())
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
        let refusal = win32(
            "read the window's title",
            "call `desktop.windows` again",
            &windows::core::Error::from_hresult(windows::core::HRESULT(-2147024891)),
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

    /// The three parts survive the path that has no error code of its
    /// own, which is the one a GDI failure takes.
    #[test]
    fn a_failure_with_no_code_of_its_own_still_carries_three_parts() {
        let refusal = last("make a bitmap for the capture", "ask for a smaller region");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        for part in ["action", "subject", "recovery"] {
            assert!(
                !error["data"][part].as_str().unwrap_or_default().is_empty(),
                "{part} is empty"
            );
        }
    }
}
