// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Retry};

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

    if let Some(gate) = refused.gate() {
        out.push_str(&format!(
            "; rule: {rule}; {violation}; instead: {alternative}",
            rule = gate.rule(),
            violation = gate.violation(),
            alternative = gate.alternative()
        ));
    }

    out.replace(['\n', '\r'], " ")
}
