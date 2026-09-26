// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Opening a Locator: `cas:` bytes and `file:` bytes at a commit
//! (runtime-SPEC section 8-29-5).
//!
//! **A content block carries no building.** The same bytes may sit in
//! the store because a confidential building put them there, and a hash
//! can be copied from anywhere, so the read bound is asked of the
//! building whose ledger line in this run's lineage referenced the
//! block. [`judged_at`] is the one place that decides which address a
//! Locator is judged at; the judgement itself stays `chosen_path::admit`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError, B3Hash, Locator, ReadVerdict};
use memory::MemoryError;

/// The address of the ledger line in this run's lineage that referenced
/// a content block, or `None` when no such line exists.
pub type BlockOwner = Arc<dyn Fn(&B3Hash) -> Result<Option<Address>, AxError> + Send + Sync>;

/// Where this city keeps content blocks, and whose each one is. One value
/// because a block read needs both: the run may write in a worktree that
/// holds no store of its own.
pub struct Blocks {
    pub store: PathBuf,
    pub owner: BlockOwner,
}

/// The text a `cas:` or `file:` argument names, or `None` when the
/// argument is not a Locator and belongs to the catalog or a path.
pub(super) fn open_locator(
    asked: &str,
    city_root: &Path,
    blocks: &Blocks,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Option<Result<String, AxError>> {
    if !(asked.starts_with("cas:") || asked.starts_with("file:")) {
        return None;
    }
    Some(read_admitted(asked, city_root, blocks, bound))
}

fn read_admitted(
    asked: &str,
    city_root: &Path,
    blocks: &Blocks,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<String, AxError> {
    let locator = Locator::parse(asked).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, "read", err.subject().to_owned()).with_recovery(
            "write a Locator as `cas:b3-<hash>` or `file:<address>@<commit>`, as the ledger \
             spells it",
        )
    })?;
    super::super::chosen_path::admit(judged_at(&locator, &blocks.owner)?.as_str(), "read", bound)?;
    let bytes = match &locator {
        Locator::Cas { hash, range } => {
            let cas = memory::Cas::open(&blocks.store).map_err(MemoryError::into_ax)?;
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
        } => memory::blob_at(city_root, *oid, address)
            .map_err(MemoryError::into_ax)?
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read",
                    format!("{asked} is not a file in that commit"),
                )
                .with_recovery("name a file the commit holds, not a directory or a later file")
            })?,
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

/// The address whose read bound decides a Locator: a `file:` at its own
/// address, a `cas:` block at the ledger line of this lineage that
/// referenced it.
///
/// # Errors
/// `E_GATE_DENIED` when no line of this run or its predecessor
/// referenced the block, and whatever the owner lookup reports.
fn judged_at(locator: &Locator, owner: &BlockOwner) -> Result<Address, AxError> {
    match locator {
        Locator::File { address, .. } => Ok(address.clone()),
        Locator::Cas { hash, .. } => owner(hash)?.ok_or_else(|| {
            AxError::failure(
                AxCode::GateDenied,
                "read",
                format!("cas:b3-{hash} was not referenced by this run or its predecessor"),
            )
            .with_recovery(
                "a content block is read only when a ledger line of this run or its \
                 predecessor referenced it, because that line says which building the bytes \
                 belong to",
            )
        }),
    }
}
