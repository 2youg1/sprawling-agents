// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A refusal as the one line a model acts on.
//!
//! The code comes first because it is what a model branches on; the
//! action and subject say what failed; the recovery says what to do. The
//! rest of an `AxError` is appended only when it changes what the model
//! should do next: the nearby names it can pick from, whether the same
//! call may go out again, and a door's three-part refusal - the rule,
//! what broke it, and what the model may do instead. Who pays
//! for a failure and which provider failed are the city's to act on,
//! and stay in the ledger.

use kernel::{AxError, Retry};

/// `E_CODE action: subject; recovery`, then each other part the model
/// can act on, parted by `; ` on the same line.
pub(super) fn line(refused: &AxError) -> String {
    let mut out = format!(
        "{code} {action}: {subject}",
        code = refused.code(),
        action = refused.action(),
        subject = refused.subject()
    );
    if !refused.recovery().is_empty() {
        out.push_str("; ");
        out.push_str(refused.recovery());
    }
    if !refused.nearby().is_empty() {
        out.push_str("; nearby: ");
        out.push_str(&refused.nearby().join(", "));
    }
    match (refused.retry(), refused.retry_after_ms()) {
        (Retry::Yes, Some(ms)) => out.push_str(&format!("; retry after {ms} ms")),
        (Retry::Yes, None) => out.push_str("; retry"),
        (Retry::Unknown, _) => out.push_str("; it may have taken effect"),
        (Retry::No, _) => {}
    }
    // A door's refusal is only whole with all three parts: the rule
    // alone tells a model it was stopped, not why or what is open to it.
    if let Some(gate) = refused.gate() {
        out.push_str(&format!(
            "; rule: {rule}; {violation}; instead: {alternative}",
            rule = gate.rule(),
            violation = gate.violation(),
            alternative = gate.alternative()
        ));
    }
    // A subject or a recovery can quote text with line breaks in it; the
    // answer stays one line so the code a model branches on is the first
    // thing on it whatever was quoted.
    out.replace(['\n', '\r'], " ")
}
