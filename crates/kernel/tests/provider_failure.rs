// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An `E_PROVIDER` error carries the kind of failure on the wire, so a
//! page can say it in the reader's language and keep the city's English
//! sentence for the fold.

#![allow(clippy::unwrap_used, reason = "test code")]

use kernel::AxError;
use serde_json::json;

#[test]
fn a_provider_error_keeps_its_failure_kind_across_the_wire() {
    let wire = json!({
        "code": "E_PROVIDER",
        "action": "call provider",
        "subject": "http://house/v1 answered 429",
        "nearby": [],
        "recovery": "check this endpoint's model name",
        "retriable": false,
        "provider": { "kind": "refused", "status": 429 },
    });
    let read = serde_json::from_value::<AxError>(wire.clone())
        .map(|err| serde_json::to_value(err).unwrap());
    assert_eq!(read.ok(), Some(wire));
}
