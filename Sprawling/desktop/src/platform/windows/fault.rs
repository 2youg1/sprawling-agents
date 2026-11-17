// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use desktop_ffi::ended::Failure;
use desktop_ffi::step::Step;

use crate::refusal::{Refusal, RefusalCode};

pub(crate) fn system(doing: &str, recovery: &str, err: winsafe::co::ERROR) -> Refusal {
    refused(doing, recovery, &err.to_string())
}

pub(crate) fn com(doing: &str, recovery: &str, err: winsafe::co::HRESULT) -> Refusal {
    refused(doing, recovery, &err.to_string())
}

pub(crate) fn automation(doing: &str, recovery: &str, err: &windows::core::Error) -> Refusal {
    refused(doing, recovery, &err.to_string())
}

pub(crate) fn leaf(doing: &str, recovery: &str, failure: Failure) -> Refusal {
    match failure {
        Failure::At { code, .. } => system(doing, recovery, code),
        Failure::Unspelled(raw) => unspelled(doing, raw),
    }
}

pub(crate) fn stray(doing: &str, step: Step) -> Refusal {
    defect(
        doing,
        &format!("its leaf stopped at {step:?}, a step this operation does not have"),
    )
}

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
