// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every machine that is not Windows.
//!
//! This server still starts, still lists its six tools and still speaks
//! the whole protocol here, so a city on another platform learns what it
//! is missing from the refusal rather than from a server that will not
//! start. The refusal names the platform, because "unavailable" without
//! a reason is a sentence nobody can act on.

use crate::refusal::{Refusal, RefusalCode};
use crate::scope::Admitted;
use serde_json::Value;

/// What one connection remembers between calls — which here is
/// nothing, because nothing happens.
///
/// It exists so that `crate::session` holds one type on every platform.
/// The alternative is a `cfg` in the session, which would put "is this
/// Windows" in two files and make the read loop something a reader has
/// to assemble mentally per platform.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Desk;

impl Desk {
    pub(crate) fn new() -> Desk {
        Desk
    }

    /// Carries out one admitted call, which here is none.
    ///
    /// The admission is taken and dropped: this arm reports no window,
    /// so it has nothing to filter, and the parameter is here because
    /// both arms present one type to `crate::session`.
    ///
    /// # Errors
    /// Refuses every call, naming this platform and the tool that was
    /// asked for.
    pub(crate) fn perform(
        &mut self,
        tool: &str,
        _arguments: &Value,
        _admitted: &Admitted<'_>,
    ) -> Result<Value, Refusal> {
        let platform = std::env::consts::OS;
        Err(Refusal::new(
            RefusalCode::ToolUnavailable,
            "use the desktop",
            format!("`{tool}` reaches the Windows desktop, and this build runs on {platform}"),
            "run this server on the Windows machine whose desktop is to be driven",
        ))
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
    use serde_json::json;

    #[test]
    fn a_platform_without_this_desktop_is_named_in_the_refusal() {
        let scope = crate::scope::Scope::parse("windows = [\"*\"]\n");
        let admitted = scope
            .admits(&crate::scope::Reach {
                tool: "desktop.windows",
                title: None,
                process: None,
            })
            .expect("an open scope admits a listing");
        let refusal = Desk::new()
            .perform("desktop.windows", &json!({}), &admitted)
            .expect_err("this platform carries out nothing");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert!(
            error["data"]["subject"]
                .as_str()
                .unwrap_or_default()
                .contains(std::env::consts::OS)
        );
    }
}
