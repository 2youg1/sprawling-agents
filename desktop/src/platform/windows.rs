// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Windows arm, which in card 7.1 carries out nothing.
//!
//! The point of this file today is that the shape is final and the
//! refusal is honest. A fabricated success would be worse than a
//! refusal, because a model reasons onward from an answer and a wrong
//! answer costs more than none (desktop-SPEC.md section 8.5, fourth
//! pair). Card 7.2 replaces the body and nothing else: the tool table,
//! the scope decision and the wire shape are already settled.

use crate::refusal::{Refusal, RefusalCode};
use serde_json::Value;

/// Carries out one admitted call.
///
/// # Errors
/// Refuses every call in this build, naming the tool that was asked for.
pub(crate) fn perform(tool: &str, _arguments: &Value) -> Result<Value, Refusal> {
    Err(Refusal::new(
        RefusalCode::ToolUnavailable,
        "use the desktop",
        format!("`{tool}` is not yet implemented in this build"),
        "this build declares the desktop tools and carries out none of them; \
         the Windows implementation arrives with card 7.2",
    ))
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
    use serde_json::json;

    #[test]
    fn this_build_refuses_every_tool_and_says_so_in_those_words() {
        let refusal = perform("desktop.act", &json!({})).expect_err("nothing is carried out yet");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap_or_default()
                .contains("card 7.2")
        );
        assert!(
            error["message"]
                .as_str()
                .unwrap_or_default()
                .contains("not yet implemented in this build")
        );
    }
}
