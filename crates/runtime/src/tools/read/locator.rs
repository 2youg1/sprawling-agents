// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Opening a Locator: `cas:` bytes and `file:` bytes at a commit
//! (runtime-SPEC section 8-29-5).
//!
//! **A content block is judged at the building it was put for.** The
//! same bytes may sit in the store because a confidential building put
//! them there, and a hash can be copied from anywhere, so the only
//! answer to whose a block is comes from the record the store wrote when
//! it took the bytes. [`judged_at`] is the one place that decides which
//! address a Locator is judged at; the judgement itself stays
//! `chosen_path::admit`.

use std::path::Path;

use kernel::{Address, AxCode, AxError, B3Hash, Locator, ReadVerdict};
use memory::MemoryError;

/// The text a `cas:` or `file:` argument names, or `None` when the
/// argument is not a Locator and belongs to the catalog or a path.
pub(super) fn open_locator(
    asked: &str,
    city_root: &Path,
    store: &Path,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Option<Result<String, AxError>> {
    if !(asked.starts_with("cas:") || asked.starts_with("file:")) {
        return None;
    }
    Some(read_admitted(asked, city_root, store, bound))
}

fn read_admitted(
    asked: &str,
    city_root: &Path,
    store: &Path,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<String, AxError> {
    let locator = Locator::parse(asked).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, "read", err.subject().to_owned()).with_recovery(
            "write a Locator as `cas:b3-<hash>` or `file:<address>@<commit>`, as the ledger \
             spells it",
        )
    })?;
    let bytes = match &locator {
        Locator::Cas { hash, range } => {
            let cas = memory::Cas::open(store).map_err(MemoryError::into_ax)?;
            let building = judged_at(
                hash,
                &cas.origins(hash).map_err(MemoryError::into_ax)?,
                bound,
            )?;
            super::super::chosen_path::admit(building.as_str(), "read", bound)?;
            match range {
                Some(range) => cas.get_range(hash, range),
                None => cas.get(hash),
            }
            .map_err(MemoryError::into_ax)?
        }
        Locator::File {
            address,
            oid,
            range: None,
        } => {
            super::super::chosen_path::admit(address.as_str(), "read", bound)?;
            memory::blob_at(city_root, *oid, address)
                .map_err(MemoryError::into_ax)?
                .ok_or_else(|| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "read",
                        format!("{asked} is not a file in that commit"),
                    )
                    .with_recovery("name a file the commit holds, not a directory or a later file")
                })?
        }
        Locator::File { range: Some(_), .. } => {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read",
                format!("{asked} carries a range, which a commit's file is not cut by"),
            )
            .with_recovery("drop the range and pass `offset` and `limit` instead"));
        }
    };
    String::from_utf8(bytes).map_err(|_| {
        AxError::failure(AxCode::InvalidArgs, "read", format!("{asked} is not text"))
            .with_recovery("read hands back text; these bytes are not UTF-8")
    })
}

/// The building whose read bound decides a `cas:` block: the first
/// building it was put for that this reader may read, or, when there is
/// none, the first it was put for, so that `admit` refuses with that
/// building's reason. A `file:` is judged at its own address and needs
/// no decision.
///
/// # Errors
/// `E_GATE_DENIED` for a block put for no building.
fn judged_at(
    hash: &B3Hash,
    origins: &[memory::BlockOrigin],
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Address, AxError> {
    origins
        .iter()
        .map(|origin| &origin.building)
        .find(|building| matches!(bound(building), ReadVerdict::Open))
        .or_else(|| origins.first().map(|origin| &origin.building))
        .cloned()
        .ok_or_else(|| {
            AxError::failure(
                AxCode::GateDenied,
                "read",
                format!("cas:b3-{hash} was put for no building"),
            )
            .with_recovery(
                "read the file the block was taken from as `file:<address>@<commit>`; a                  content block is read only at the building it was put for",
            )
        })
}
