// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What an attach read about each model, kept in the CAS so that a
//! book folded again learns it back (`crates/gateway/spec/Endpoint/Models.lean`
//! §8-10, kernel D57).
//!
//! The endpoint book folds an `endpoint_attached` line to ids only,
//! because a fold does no I/O. Whoever holds the CAS writes the facts
//! before the line and hands their bytes back to the book after it; the
//! format is the gateway's (`AttachedEndpoint::facts_bytes`,
//! `EndpointBook::learn`), and this module only moves bytes.

use std::path::Path;

use kernel::event::record::EndpointAttached;
use kernel::{AxError, EventKind, EventRecord};
use storage::{Cas, StorageError};

/// Writes `endpoint`'s facts into the CAS and names the blob on it,
/// before the line that carries the name is written.
///
/// # Errors
/// Propagates facts that do not encode and a CAS that refuses the write.
pub(crate) fn keep(
    cas: &mut Cas,
    endpoint: &mut gateway::AttachedEndpoint,
) -> Result<(kernel::B3Hash, Vec<u8>), AxError> {
    let bytes = endpoint.facts_bytes()?;
    let blob = cas.put(&bytes).map_err(StorageError::into_ax)?;
    endpoint.facts_blob = Some(blob);
    Ok((blob, bytes))
}

/// The book with every blob it names learned back, for a book folded
/// from a snapshot or from genesis.
///
/// # Errors
/// Propagates a blob the CAS cannot return and bytes that are not model
/// facts: the book would otherwise offer less than the history says.
pub(crate) fn learned(
    mut book: gateway::EndpointBook,
    cas: &Cas,
) -> Result<gateway::EndpointBook, AxError> {
    let blobs: Vec<kernel::B3Hash> = book
        .endpoints()
        .filter_map(|endpoint| endpoint.facts_blob)
        .collect();
    for blob in blobs {
        let bytes = cas.get(&blob).map_err(StorageError::into_ax)?;
        book.learn(&blob, &bytes)?;
    }
    Ok(book)
}

/// Folds one line into `book`, and learns the facts an
/// `endpoint_attached` line names from the city's CAS.
///
/// # Errors
/// Propagates the book's own refusal, an unreadable line and a blob the
/// CAS cannot return.
pub(crate) fn apply(
    book: &mut gateway::EndpointBook,
    city_root: &Path,
    record: &EventRecord,
) -> Result<(), AxError> {
    book.apply(record)?;
    if record.kind() != EventKind::EndpointAttached {
        return Ok(());
    }
    let attached: EndpointAttached = record.data().read()?;
    let Some(blob) = attached.facts_blob else {
        return Ok(());
    };
    let cas = Cas::open(&kernel::layout::CityLayout::new(city_root).cas())
        .map_err(StorageError::into_ax)?;
    let bytes = cas.get(&blob).map_err(StorageError::into_ax)?;
    book.learn(&blob, &bytes)
}
