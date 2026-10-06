// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! An endpoint's tuning as the wire carries it and as the endpoint book
//! keeps it, translated in both directions in one place: the worker
//! reads a form's tuning in, and the views read the book's back out for
//! the settings page (`crates/wire/spec/Answer/Endpoints.lean` §8-85).
//!
//! Apart from the worker because both of them read it, and the views
//! never name the worker.

use kernel::AxError;

/// The frame's tuning as the book keeps it.
///
/// Two readings happen here and nowhere else. **A zero is absence**: a
/// deadline no request can meet is what an empty box reaches the wire
/// as, and reading it as a figure would make every call fail instantly
/// for somebody who cleared a field. **`stream_idle_timeout_ms` becomes
/// a deadline for the whole streamed request**, which is what this
/// city's blocking transport can enforce; the wire keeps the name the
/// person's own `config.toml` uses, and `gateway` states what it does
/// with it (`crates/gateway/Spec.lean` §8-16).
///
/// A row with no name and a pointer with no path are dropped: a form
/// that keeps an empty row open while somebody types is a form whose
/// half-written rows must not reach a provider.
/// # Errors
/// `E_CONFIG_INVALID` naming the header whose value reads as a
/// credential without being a vault reference.
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
    })
}

/// The book's tuning in the frame's shape: [`tuning_of`] read backwards,
/// so the settings page can send an endpoint back whole with only its
/// account list changed (`crates/wire/spec/Answer/Endpoints.lean` §8-85).
///
/// Beside [`tuning_of`] so a field added to one direction meets the other
/// in the same file. Every tuning [`tuning_of`] accepts comes back from
/// `tuning_of(tuning_as_attached(..))` unchanged: the readings it drops
/// (a zero, a blank row) are already gone from the book.
pub(crate) fn tuning_as_attached(tuning: &gateway::EndpointTuning) -> wire::EndpointTuning {
    wire::EndpointTuning {
        accounts: tuning.accounts.clone(),
        ..wire::EndpointTuning::default()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;
