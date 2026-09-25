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

use std::path::{Path, PathBuf};
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

/// Where an admitted address really lands on disk, judged again there.
///
/// The grammar judged the address the model wrote, and the disk opens
/// whatever every link on the way leads to: a link in an open building
/// can point into a confidential one, into a reserved subtree, or out of
/// the city. So the real path is resolved and the address it names in
/// this city goes through [`admit`] a second time; the one judgement
/// stays the only one. A path that does not exist comes back as the
/// grammar placed it, and the open that follows reports it missing.
///
/// # Errors
/// `E_GATE_DENIED` when the real path is outside the city or [`admit`]
/// refuses the address it names; `E_STORAGE_FATAL` when the path exists
/// and its real location cannot be resolved.
pub(crate) fn land(
    city_root: &Path,
    addr: &Address,
    action: &'static str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<PathBuf, AxError> {
    let written = addr
        .as_str()
        .split('/')
        .fold(city_root.to_path_buf(), |path, segment| path.join(segment));
    let unresolved = |err: std::io::Error| {
        AxError::failure(
            AxCode::StorageFatal,
            action,
            format!(
                "{}: its real location did not resolve ({err})",
                addr.as_str()
            ),
        )
        .with_recovery("a person has to repair the path or the link on it")
    };
    let real = match std::fs::canonicalize(&written) {
        Ok(real) => real,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(written),
        Err(err) => return Err(unresolved(err)),
    };
    let root = std::fs::canonicalize(city_root).map_err(unresolved)?;
    let outside = || {
        AxError::failure(
            AxCode::GateDenied,
            action,
            format!("{} leads out of the city through a link", addr.as_str()),
        )
        .with_recovery("name a path whose every link stays inside the city")
    };
    let inside = real
        .strip_prefix(&root)
        .map_err(|_| outside())?
        .iter()
        .map(std::ffi::OsStr::to_str)
        .collect::<Option<Vec<&str>>>()
        .ok_or_else(outside)?
        .join("/");
    // A path with no link on it names the address already admitted, and
    // asking the read bound twice would read a building's rules twice.
    if inside != addr.as_str() {
        admit(&inside, action, bound)?;
    }
    Ok(real)
}

/// What a walk may do at one of its entries.
pub(crate) enum Walked {
    /// A directory reached without a link: its entries are walked.
    Directory(PathBuf),
    /// A file, or the file an admitted link lands on: it is scanned.
    File(PathBuf),
    /// A link [`land`] refuses, or a link to a directory, which is not
    /// entered because a walk through links can come back to where it
    /// started. Passed over without a word, as a closed building is.
    Passed,
}

/// Judges one entry of a walk, asking the disk once about an entry that
/// is not a link. An entry the disk will not describe is handed on as a
/// file, so the open that follows names why it could not be looked at.
pub(crate) fn walked(
    city_root: &Path,
    path: PathBuf,
    rel: &str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Walked {
    let Ok(kind) = std::fs::symlink_metadata(&path).map(|meta| meta.file_type()) else {
        return Walked::File(path);
    };
    if kind.is_dir() {
        return Walked::Directory(path);
    }
    if !kind.is_symlink() {
        return Walked::File(path);
    }
    let landed = Address::parse(rel).and_then(|addr| land(city_root, &addr, "search", bound));
    match landed {
        Ok(real) if real.is_file() => Walked::File(real),
        Ok(_) | Err(_) => Walked::Passed,
    }
}

/// Places a link at `link` leading to the directory `target`, on a
/// machine that has no symlink privilege: Windows gets a junction, which
/// `symlink_metadata` reports through the same name-surrogate predicate
/// a symlink is reported by.
#[cfg(all(test, windows))]
#[expect(clippy::unwrap_used, reason = "test fixture")]
pub(super) fn make_link(link: &std::path::Path, target: &std::path::Path) {
    let made = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap();
    assert!(
        made.status.success(),
        "the fixture could not make a link: {}",
        String::from_utf8_lossy(&made.stdout)
    );
    let meta = std::fs::symlink_metadata(link).unwrap();
    assert!(meta.file_type().is_symlink(), "{link:?} is not a link");
}

#[cfg(all(test, unix))]
#[expect(clippy::unwrap_used, reason = "test fixture")]
pub(super) fn make_link(link: &std::path::Path, target: &std::path::Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
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
