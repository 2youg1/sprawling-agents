// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one judgement a path chosen by a model gets.
//!
//! Two tools take a path from the model — `read` opens it, `search`
//! bounds a walk with it — and both have to answer the same three
//! questions before touching a disk: does this parse as an address of
//! this city, does it reach a reserved subtree, and may this run's
//! building read it. A second copy of those answers would be a second
//! authority, and the copy is always the one that stops being updated;
//! so the refusals live here, reservedness stays in
//! `kernel::Address::is_reserved`, and the read bound in
//! `kernel::address::may_read`.
//!
//! The order is fixed and cheapest first: the grammar and the reserved
//! subtree touch no disk, and the read bound may read one building's
//! rules.
//!
//! A catalog name is not judged here. Admission for a catalog entry
//! happened when a person wrote the building's reading room, which is
//! why a skill inside reserved space is still handed over: `read`
//! resolves the catalog first and only unresolved names reach this.

use std::sync::Arc;

use kernel::{Address, AxCode, AxError, ReadVerdict};

/// What one run may read: the read bound closed over the reader's
/// building and the city's rules. The assembly builds it once per run
/// and hands a clone to `read` and to `search`, so both ask the same
/// question of the same rules.
pub type ReadBound = Arc<dyn Fn(&Address) -> ReadVerdict + Send + Sync>;

/// The address a model may act on, or the refusal that says why not.
///
/// `action` names the tool in both errors, so a refusal a model reads
/// says which of its own calls was turned down.
///
/// # Errors
/// `E_INVALID_ARGS` when the string is not a canonical city-relative
/// address, `E_GATE_DENIED` when it reaches a reserved subtree or a
/// building the read bound closes.
pub(crate) fn admit(
    asked: &str,
    action: &'static str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Address, AxError> {
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
            "a reserved subtree holds what governs a scope or keeps the repository itself \
             (`.sprawling`, `.git`), and no run reads its own governance; ask for a skill by \
             its catalog name instead, or name a path outside it",
        ));
    }
    match bound(&addr) {
        ReadVerdict::Open => Ok(addr),
        ReadVerdict::Confidential => Err(AxError::failure(
            AxCode::GateDenied,
            action,
            format!("{asked} is inside a confidential building"),
        )
        .with_recovery(
            "a confidential building is read by its own residents and nobody else; signal a \
             resident there with the question, or work from what your own building holds",
        )),
        ReadVerdict::RulesUnreadable(unread) => Err(AxError::failure(
            AxCode::GateDenied,
            action,
            format!(
                "{asked}: the rules of its building did not read ({})",
                unread.subject()
            ),
        )
        .with_recovery(
            "a building whose rules cannot be read is closed to every other building, because \
             those rules may say it is confidential; a person has to fix them before anyone \
             outside reads there",
        )),
    }
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

    /// A bound under which every address is open, so a test about the
    /// grammar or the reserved subtree judges nothing else.
    fn open(_: &Address) -> ReadVerdict {
        ReadVerdict::Open
    }

    #[test]
    fn a_reserved_subtree_is_closed_at_every_depth() {
        for asked in [
            ".sprawling",
            ".sprawling/ledger/0001.jsonl",
            "lab/.sprawling/RULES.toml",
            "lab/room1/.sprawling/notes/one.md",
        ] {
            let err = admit(asked, "read", &open).unwrap_err();
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
            let err = admit(asked, "search", &open).unwrap_err();
            assert_eq!(err.code(), &AxCode::InvalidArgs, "{asked} was allowed");
            assert_eq!(err.action(), "search");
        }
    }

    #[test]
    fn an_ordinary_path_comes_back_as_the_address_it_names() {
        let addr = admit("lab/room1/Memo.md", "read", &open).unwrap();
        assert_eq!(addr.as_str(), "lab/room1/Memo.md");
    }

    /// Both ways the read bound closes come back as the gate's refusal,
    /// each with its own next step: a confidential building is asked
    /// through a resident, and rules that do not read wait for a person.
    #[test]
    fn a_building_the_read_bound_closes_is_refused_with_its_own_way_out() {
        let unread = AxError::failure(AxCode::ConfigInvalid, "read a building's rules", "annex")
            .with_recovery("fix the file");
        for (verdict, says) in [
            (ReadVerdict::Confidential, "signal a resident"),
            (ReadVerdict::RulesUnreadable(unread), "a person has to fix"),
        ] {
            let err = admit("vault/room1/notes.md", "read", &|_| verdict.clone()).unwrap_err();
            assert_eq!(err.code(), &AxCode::GateDenied);
            assert!(err.recovery().contains(says), "{}", err.recovery());
        }
    }
}
