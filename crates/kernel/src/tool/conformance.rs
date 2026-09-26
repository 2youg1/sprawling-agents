// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One assertion suite for every tool implementation (V3).

use super::{Tool, ToolCall, ToolName};
use crate::error::AxCode;
use crate::event::Payload;

/// Asserts the meta is complete and the identity is fail-closed:
/// a wrong-name call is refused with `E_INVALID_ARGS`, and the tool
/// still answers after refusing (no poisoned state).
#[cfg(feature = "conformance")]
#[allow(
    clippy::panic,
    clippy::expect_used,
    reason = "conformance suites assert by panicking; they are dev-only by feature"
)]
pub fn assert_tool_conformance<T: Tool>(tool: &mut T) {
    let meta = tool.meta().clone();
    assert!(
        !meta.name.as_str().is_empty(),
        "tool name must be non-empty"
    );
    assert!(
        !meta.disclosure.is_empty(),
        "disclosure must answer what-and-when in one breath"
    );
    let wrong = ToolName::parse("no_such_tool_name").expect("literal is well-formed");
    assert_ne!(
        wrong, meta.name,
        "conformance probe name collides with the tool under test"
    );
    let refused = tool.invoke(&ToolCall {
        id: "conf-mismatch".to_owned(),
        name: wrong,
        args: Payload::empty(),
    });
    match refused {
        Err(err) => assert_eq!(
            err.code(),
            &AxCode::InvalidArgs,
            "wrong-name call must be E_INVALID_ARGS"
        ),
        Ok(_) => panic!("a tool must refuse a call bearing another tool's name"),
    }
    let meta_after = tool.meta();
    assert_eq!(
        meta_after.name, meta.name,
        "a refusal must not poison the tool"
    );
}
