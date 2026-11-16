// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::EndpointAttached;
use kernel::{AxError, EventKind, EventRecord};
use storage::{Cas, StorageError};

pub(crate) fn keep(
    cas: &mut Cas,
    endpoint: &mut gateway::AttachedEndpoint,
) -> Result<(kernel::B3Hash, Vec<u8>), AxError> {
    let bytes = endpoint.facts_bytes()?;
    let blob = cas.put(&bytes).map_err(StorageError::into_ax)?;
    endpoint.facts_blob = Some(blob);
    Ok((blob, bytes))
}

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
