// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::{EndpointProbed, ProbeFailure};
use kernel::{AxCode, AxError, Payload, Proxying, Reach};

pub(super) struct Probing {
    pub(super) reach: Reach,

    pub(super) served: Result<Vec<gateway::ModelFacts>, AxError>,
}

pub(super) fn reach_of(
    base_url: &str,
    proxying: Proxying,
    monotonic: fn() -> std::time::Instant,
) -> Result<Reach, AxError> {
    let client = gateway::client_for(proxying, base_url)
        .build()
        .map_err(|err| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "measure reachability",
                err.to_string(),
            )
            .with_recovery("restart the server; this machine refused to build an HTTP client")
        })?;
    let before = monotonic();
    let reading = gateway::reach(&client, proxying, base_url, 0);
    let spent = monotonic().saturating_duration_since(before).as_millis();
    Ok(Reach {
        elapsed_ms: u64::try_from(spent).unwrap_or(u64::MAX),
        ..reading
    })
}

pub(super) fn probed_payload(
    name: &str,
    base_url: &str,
    found: Probing,
) -> Result<Payload, AxError> {
    let (facts, failed) = match found.served {
        Ok(rows) => (rows, None),
        Err(err) => (
            Vec::new(),
            Some(ProbeFailure {
                code: err.code().as_str().to_owned(),
                subject: err.subject().to_owned(),
            }),
        ),
    };
    Payload::of(&EndpointProbed {
        name: name.to_owned(),
        base_url: base_url.to_owned(),
        reach: found.reach,
        models: facts.iter().map(|row| row.id.clone()).collect(),
        facts,
        failed,
    })
}
