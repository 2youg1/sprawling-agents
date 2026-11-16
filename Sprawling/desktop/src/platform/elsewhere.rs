// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::answer::Answer;
use crate::refusal::{Refusal, RefusalCode};
use crate::scope::Admitted;
use crate::tools::ToolName;
use serde_json::Value;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Desk;

impl Desk {
    pub(crate) fn new() -> Desk {
        Desk
    }

    pub(crate) fn perform(
        &mut self,
        tool: ToolName,
        _arguments: &Value,
        _admitted: &Admitted<'_>,
    ) -> Result<Answer, Refusal> {
        let platform = std::env::consts::OS;
        let tool = tool.as_str();
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
                tool: ToolName::Windows,
                title: None,
                process: None,
            })
            .expect("an open scope admits a listing");
        let refusal = Desk::new()
            .perform(ToolName::Windows, &json!({}), &admitted)
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
