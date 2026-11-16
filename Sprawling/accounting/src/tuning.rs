// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

pub(crate) fn tuning_of(wire: wire::EndpointTuning) -> Result<gateway::EndpointTuning, AxError> {
    let stated = |figure: Option<u64>| figure.filter(|ms| *ms > 0);
    let mut extra_headers = Vec::new();
    for row in wire.headers {
        let name = row.name.trim().to_owned();
        if name.is_empty() {
            continue;
        }
        extra_headers.push((
            name.clone(),
            gateway::HeaderValue::parse(&name, &row.value)?,
        ));
    }
    gateway::validate_accounts(wire.accounts.as_deref())?;
    Ok(gateway::EndpointTuning {
        accounts: wire.accounts,
        label: wire
            .label
            .map(|given| given.trim().to_owned())
            .filter(|given| !given.is_empty()),
        timeout_ms: stated(wire.timeout_ms),
        request_max_retries: wire
            .request_max_retries
            .map_or(kernel::Retries::UntilHalted, kernel::Retries::AtMost),
        stream_idle_timeout_ms: stated(wire.stream_idle_timeout_ms),
        extra_headers,
        overrides: wire
            .overrides
            .into_iter()
            .map(|row| (row.pointer.trim().to_owned(), row.value))
            .filter(|(pointer, _)| pointer.starts_with('/'))
            .collect(),
        proxying: wire.proxying.unwrap_or_default(),
        max_in_flight: wire
            .max_in_flight
            .filter(|ceiling| *ceiling > 0)
            .map(gateway::MaxInFlight::try_from)
            .transpose()?,
        account_retries: wire.account_retries,
        generation: wire.generation,
    })
}

pub(crate) fn tuning_as_attached(tuning: &gateway::EndpointTuning) -> wire::EndpointTuning {
    wire::EndpointTuning {
        accounts: tuning.accounts.clone(),
        label: tuning.label.clone(),
        timeout_ms: tuning.timeout_ms,
        request_max_retries: tuning.request_max_retries.stated(),
        stream_idle_timeout_ms: tuning.stream_idle_timeout_ms,
        headers: tuning
            .extra_headers
            .iter()
            .map(|(name, value)| wire::HeaderPair {
                name: name.clone(),
                value: value.spelled(),
            })
            .collect(),
        overrides: tuning
            .overrides
            .iter()
            .map(|(pointer, value)| wire::BodyOverride {
                pointer: pointer.clone(),
                value: value.clone(),
            })
            .collect(),
        proxying: Some(tuning.proxying),
        max_in_flight: tuning.max_in_flight.map(gateway::MaxInFlight::get),
        account_retries: tuning.account_retries,
        generation: tuning.generation.clone(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
