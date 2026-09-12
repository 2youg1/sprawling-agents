// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a probe found, and how the city writes it down.
//!
//! **A probe's answer is a reading, not a success or a failure.** A
//! host that does not resolve, a socket nothing answers, a certificate
//! this machine does not trust and a provider that says 401 are four
//! different next steps for the person filling in the form, and all four
//! used to arrive as one sentence from a transport library. So every
//! probe records where the call stopped, stage by stage, and a probe
//! that never reached a model list records that beside the stage that
//! stopped it rather than instead of it.
//!
//! The vocabulary of the stages is `kernel::reach`; what a socket can
//! find out is `gateway::reach`; what this module owns is the record
//! the page folds.

use kernel::{AxCode, AxError, Payload, Reach};
use serde_json::{Map, Value};

use crate::assembly::now_ms;

/// What one probe learned about one endpoint.
pub(super) struct Probing {
    /// Where a plain request to the base URL stopped.
    pub(super) reach: Reach,
    /// What the model list said, or why it could not be read.
    pub(super) served: Result<Vec<gateway::ModelFacts>, AxError>,
}

/// One staged reading of the base URL a person entered.
///
/// The client is built here rather than reused from the probe itself:
/// this asks the base URL, not the model list, and it must ask with the
/// same proxy decision the call will make. A city that cannot build an
/// HTTP client at all has no reading to report, and says so.
///
/// # Errors
/// A transport this machine will not construct, or a clock that reads
/// before the unix epoch.
pub(super) fn reach_of(base_url: &str) -> Result<Reach, AxError> {
    let mut builder = reqwest::blocking::Client::builder();
    if gateway::is_local(base_url) {
        builder = builder.no_proxy();
    }
    let client = builder.build().map_err(|err| {
        AxError::failure(
            AxCode::ConfigInvalid,
            "measure reachability",
            err.to_string(),
        )
        .with_recovery("restart the server; this machine refused to build an HTTP client")
    })?;
    let before = now_ms()?;
    let reading = gateway::reach(&client, base_url, 0);
    let after = now_ms()?;
    Ok(Reach {
        elapsed_ms: after.value().saturating_sub(before.value()),
        ..reading
    })
}

/// The `endpoint_probed` record.
///
/// It carries three things a page needs and one it can compute: the ids,
/// so a client that only wants names has them; the facts, so a table can
/// show a window and a ceiling the provider itself stated; the staged
/// reading, so a refusal is shown beside the field that caused it; and,
/// when the list could not be read, the code and subject of the failure.
/// Nothing here invents a figure: a fact the row did not state is absent
/// from the record as well.
///
/// # Errors
/// A payload the ledger will not take.
pub(super) fn probed_payload(
    name: &str,
    base_url: &str,
    found: &Probing,
) -> Result<Payload, AxError> {
    let mut map = Map::new();
    map.insert("name".to_owned(), Value::String(name.to_owned()));
    map.insert("base_url".to_owned(), Value::String(base_url.to_owned()));
    map.insert(
        "reach".to_owned(),
        serde_json::to_value(&found.reach).map_err(|err| {
            AxError::failure(
                AxCode::InvalidArgs,
                "encode a reach report",
                err.to_string(),
            )
        })?,
    );
    match &found.served {
        Ok(rows) => {
            map.insert(
                "models".to_owned(),
                Value::Array(
                    rows.iter()
                        .map(|row| Value::String(row.id.clone()))
                        .collect(),
                ),
            );
            map.insert(
                "facts".to_owned(),
                serde_json::to_value(rows).map_err(|err| {
                    AxError::failure(AxCode::InvalidArgs, "encode model facts", err.to_string())
                })?,
            );
        }
        Err(err) => {
            // An empty list rather than no key: a page that reads
            // `models` gets the same shape whether the list was empty or
            // unreadable, and reads `failed` to tell the two apart.
            map.insert("models".to_owned(), Value::Array(Vec::new()));
            map.insert("facts".to_owned(), Value::Array(Vec::new()));
            let mut why = Map::new();
            why.insert(
                "code".to_owned(),
                Value::String(err.code().as_str().to_owned()),
            );
            why.insert(
                "subject".to_owned(),
                Value::String(err.subject().to_owned()),
            );
            map.insert("failed".to_owned(), Value::Object(why));
        }
    }
    Payload::new(map)
}
