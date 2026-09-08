// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one judgement a path chosen by a model gets.
//!
//! Two tools take a path from the model — `read` opens it, `search`
//! bounds a walk with it — and both have to answer the same question
//! before touching a disk: does this parse as an address of this city,
//! and does it reach a reserved subtree. A second copy of that answer
//! would be a second authority, and the copy is always the one that
//! stops being updated; so the answer lives here and reservedness
//! itself stays where it already was, in `kernel::Address::is_reserved`.
//!
//! A catalog name is not judged here. Admission for a catalog entry
//! happened when a person wrote the building's reading room, which is
//! why a skill inside reserved space is still handed over: `read`
//! resolves the catalog first and only unresolved names reach this.

use kernel::{Address, AxCode, AxError};

/// The address a model may act on, or the refusal that says why not.
///
/// `action` names the tool in both errors, so a refusal a model reads
/// says which of its own calls was turned down.
///
/// # Errors
/// `E_INVALID_ARGS` when the string is not a canonical city-relative
/// address, `E_GATE_DENIED` when it reaches a reserved subtree.
pub(crate) fn admit(asked: &str, action: &'static str) -> Result<Address, AxError> {
    let addr = Address::parse(asked).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            action,
            format!("{asked}: {}", err.subject()),
        )
        .with_recovery(
            "pass a city-relative path with no `..`, no leading slash and no empty segment",
        )
    })?;
    if addr.is_reserved() {
        return Err(AxError::failure(
            AxCode::GateDenied,
            action,
            format!("{asked} is inside a reserved subtree"),
        )
        .with_recovery(
            "a `.sprawling` directory holds what governs a scope, and no run reads its own \
             governance; ask for a skill by its catalog name instead, or name a path outside it",
        ));
    }
    Ok(addr)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn a_reserved_subtree_is_closed_at_every_depth() {
        for asked in [
            ".sprawling",
            ".sprawling/ledger/0001.jsonl",
            "lab/.sprawling/BUILDING.md",
            "lab/room1/.sprawling/notes/one.md",
        ] {
            let err = admit(asked, "read").unwrap_err();
            assert_eq!(err.code(), &AxCode::GateDenied, "{asked} was allowed");
            assert!(
                !err.recovery().is_empty(),
                "{asked} refused with no way out"
            );
        }
    }

    #[test]
    fn a_path_that_is_not_an_address_is_the_callers_mistake() {
        for asked in ["", "/etc/passwd", "lab/../.sprawling/CONFIG.toml", "lab//x"] {
            let err = admit(asked, "search").unwrap_err();
            assert_eq!(err.code(), &AxCode::InvalidArgs, "{asked} was allowed");
            assert_eq!(err.action(), "search");
        }
    }

    #[test]
    fn an_ordinary_path_comes_back_as_the_address_it_names() {
        let addr = admit("lab/room1/Memo.md", "read").unwrap();
        assert_eq!(addr.as_str(), "lab/room1/Memo.md");
    }
}
