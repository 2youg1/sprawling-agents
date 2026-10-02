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
//!
//! `crates/runtime/spec/Tools/ChosenPath.lean` proves what the judgement
//! must hold: an admitted path is open and outside every reserved
//! subtree, and `land` judges where the disk really lands (§8-30-1).

use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError, ReadVerdict};
use same_file::Handle;

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
             those rules may say it is confidential; the User has to fix them before anyone \
             outside reads there",
        )),
    }
}

/// The city spelling of a path a model wrote: an absolute path inside
/// the city becomes the address it names there, and anything else comes
/// back as written.
///
/// The page inserts a dropped file as this machine's path to it, and a
/// model copies the paths it is shown. The answer is only a spelling:
/// it still goes through [`admit`] and [`land`], so the reserved
/// subtree, the read bound and every link keep their one judgement.
///
/// # Errors
/// `E_GATE_DENIED` when the path really lands outside the city;
/// whatever [`real_location`] refuses the path or the city root with.
pub(crate) fn within_city<'a>(
    city_root: &Path,
    asked: &'a str,
    action: &'static str,
) -> Result<Cow<'a, str>, AxError> {
    let written = Path::new(asked);
    if !written.is_absolute() {
        return Ok(Cow::Borrowed(asked));
    }
    let root = real_location(city_root, action, asked)?.into_path();
    let located = real_location(written, action, asked)?;
    let outside = || {
        AxError::failure(
            AxCode::GateDenied,
            action,
            format!("{asked} is outside the city"),
        )
        .with_recovery(
            "`read`, `search` and `edit` reach only files inside the city; to read a file \
             elsewhere on this machine, run a command with `exec`",
        )
    };
    spelled_under(&root, located.path())
        .map(Cow::Owned)
        .ok_or_else(outside)
}

/// The city spelling of `real` under the city's real root: its segments
/// joined by `/`. `None` when it is not under the root, or when a
/// segment is not text and so names no address.
fn spelled_under(root: &Path, real: &Path) -> Option<String> {
    real.strip_prefix(root)
        .ok()?
        .iter()
        .map(std::ffi::OsStr::to_str)
        .collect::<Option<Vec<&str>>>()
        .map(|segments| segments.join("/"))
}

/// Where an admitted address really lands on disk, judged again there.
///
/// The grammar judged the address the model wrote, and the disk opens
/// whatever every link on the way leads to: a link in an open building
/// can point into a confidential one, into a reserved subtree, or out of
/// the city. So the real path is resolved and the address it names in
/// this city goes through [`admit`] a second time; the one judgement
/// stays the only one. A path that does not exist is judged at the
/// place [`real_location`] puts it and comes back [`Located::Absent`],
/// which no caller opens.
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
) -> Result<Located, AxError> {
    let written = addr
        .as_str()
        .split('/')
        .fold(city_root.to_path_buf(), |path, segment| path.join(segment));
    let located = real_location(&written, action, addr.as_str())?;
    let root = real_location(city_root, action, addr.as_str())?.into_path();
    let outside = || {
        AxError::failure(
            AxCode::GateDenied,
            action,
            format!("{} leads out of the city through a link", addr.as_str()),
        )
        .with_recovery("name a path whose every link stays inside the city")
    };
    let inside = spelled_under(&root, located.path()).ok_or_else(outside)?;
    // A path with no link on it names the address already admitted, and
    // asking the read bound twice would read a building's rules twice.
    if inside != addr.as_str() {
        admit(&inside, action, bound)?;
    }
    Ok(located)
}

/// Whether the file `opened` at the judged real location `judged` is
/// still the file judged there: the location must still resolve to
/// itself, so no link was swapped onto it after the judgement, and the
/// file at it now must be the one opened, so no link was there during
/// the open and gone again before the resolution. The refusal does not
/// say where a swapped link leads. Every caller that opens what [`land`]
/// answered asks this before reading a byte.
///
/// # Errors
/// `E_GATE_DENIED`, naming `action`, when either no longer holds.
pub(crate) fn still_judged(
    asked: &str,
    judged: &Path,
    opened: &Handle,
    action: &'static str,
) -> Result<(), AxError> {
    let changed = |why: &str| {
        AxError::failure(
            AxCode::GateDenied,
            action,
            format!("{asked} changed after it was judged: {why}"),
        )
        .with_recovery("ask again once nothing is moving the directories on its path")
    };
    match std::fs::canonicalize(judged) {
        Ok(real) if real == judged => {}
        Ok(_) => return Err(changed("a link now stands on its path")),
        Err(err) => return Err(changed(&err.to_string())),
    }
    match Handle::from_path(judged) {
        Ok(now) if now == *opened => Ok(()),
        Ok(_) => Err(changed("another file stands there now")),
        Err(err) => Err(changed(&err.to_string())),
    }
}

/// Where a path really lands, and whether the disk had anything there
/// when it was asked.
pub(crate) enum Located {
    /// Every segment exists: the path with every link on it resolved.
    Present(PathBuf),
    /// The deepest existing ancestor resolved, the segments below it
    /// appended as written. Nothing was there to judge, so a caller
    /// reports the miss and never opens it: a link placed at an absent
    /// segment after this answer would lead the open wherever it points.
    Absent(PathBuf),
}

impl Located {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Located::Present(path) | Located::Absent(path) => path,
        }
    }

    pub(crate) fn into_path(self) -> PathBuf {
        match self {
            Located::Present(path) | Located::Absent(path) => path,
        }
    }
}

/// Where `written` really lands, every link on it resolved, whether or
/// not its last segments exist.
///
/// The disk resolves the deepest ancestor that exists; the segments
/// below it exist nowhere, so no link hides in them, and they are
/// appended as written, and the answer says so: [`Located::Absent`]. An
/// absent file behind a link is therefore judged under the link's
/// target, as a present one is, and never at the spelling the link
/// stood on. Every caller that judges where a path lands asks this
/// function, so an absent file has one answer.
///
/// # Errors
/// `E_GATE_DENIED` when a segment below that ancestor is not a plain
/// name: appended to a resolved directory, a `..` steps back out of it
/// at a place the disk never looked. `E_STORAGE_FATAL` when an entry on
/// the path exists and does not resolve, a link whose target is gone
/// among them.
pub(crate) fn real_location(
    written: &Path,
    action: &'static str,
    subject: &str,
) -> Result<Located, AxError> {
    let unresolved = |err: std::io::Error| {
        AxError::failure(
            AxCode::StorageFatal,
            action,
            format!("{subject}: its real location did not resolve ({err})"),
        )
        .with_recovery("the User has to repair the path or the link on it")
    };
    for existing in written.ancestors() {
        match std::fs::canonicalize(existing) {
            Ok(real) if existing == written => return Ok(Located::Present(real)),
            Ok(real) => {
                return written
                    .components()
                    .skip(existing.components().count())
                    .try_fold(real, |path, segment| match segment {
                        Component::Normal(name) => Ok(path.join(name)),
                        Component::Prefix(_)
                        | Component::RootDir
                        | Component::CurDir
                        | Component::ParentDir => Err(AxError::failure(
                            AxCode::GateDenied,
                            action,
                            format!("{subject} climbs out of the directory it names"),
                        )
                        .with_recovery("name a path made of plain segments")),
                    })
                    .map(Located::Absent);
            }
            Err(err) if err.kind() == ErrorKind::NotFound && no_entry(existing) => {}
            Err(err) => return Err(unresolved(err)),
        }
    }
    Err(unresolved(ErrorKind::NotFound.into()))
}

/// Whether nothing at all is at `path`, not even a link whose target is
/// gone: only such a segment may be appended unresolved.
fn no_entry(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(err) if err.kind() == ErrorKind::NotFound)
}

/// What a walk may do at one of its entries.
pub(crate) enum Walked {
    /// A directory reached without a link: its entries are walked.
    Directory(PathBuf),
    /// A file, or the file an admitted link lands on: it is scanned.
    File(PathBuf),
    /// A link [`land`] refuses at the gate, or a link to a directory,
    /// which is not entered because a walk through links can come back
    /// to where it started. Passed over without a word, as a closed
    /// building is.
    Passed,
}

/// Judges one entry of a walk, asking the disk once about an entry that
/// is not a link. An entry the disk will not describe is handed on as a
/// file, so the open that follows names why it could not be looked at.
///
/// # Errors
/// Whatever [`land`] refuses a link with other than `E_GATE_DENIED`:
/// a link whose real location does not resolve is a place the walk
/// could not look, not a closed building, and the walker reports it.
pub(crate) fn walked(
    city_root: &Path,
    path: PathBuf,
    rel: &str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Walked, AxError> {
    let Ok(kind) = std::fs::symlink_metadata(&path).map(|meta| meta.file_type()) else {
        return Ok(Walked::File(path));
    };
    if kind.is_dir() {
        return Ok(Walked::Directory(path));
    }
    if !kind.is_symlink() {
        return Ok(Walked::File(path));
    }
    match Address::parse(rel).and_then(|addr| land(city_root, &addr, "search", bound)) {
        Ok(Located::Present(real)) if real.is_file() => Ok(Walked::File(real)),
        Ok(Located::Present(_) | Located::Absent(_)) => Ok(Walked::Passed),
        Err(refused) if refused.code() == &AxCode::GateDenied => Ok(Walked::Passed),
        Err(unresolved) => Err(unresolved),
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

    /// A dropped file reaches the conversation as this machine's path
    /// to it, and it lies inside the city: the model that copies it is
    /// asking for that file's address, in either separator.
    #[test]
    fn an_absolute_path_inside_the_city_comes_back_as_its_address() {
        let city = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(city.path().join("hall").join("dropped")).unwrap();
        std::fs::write(
            city.path().join("hall").join("dropped").join("plan.md"),
            "x",
        )
        .unwrap();
        let native = city.path().join("hall").join("dropped").join("plan.md");
        let native = native.to_str().unwrap();
        let forward = native.replace('\\', "/");
        let spelled = [native, forward.as_str()].map(|asked| {
            within_city(city.path(), asked, "read")
                .unwrap()
                .into_owned()
        });
        assert_eq!(spelled, ["hall/dropped/plan.md", "hall/dropped/plan.md"]);
    }

    #[test]
    fn an_absolute_path_outside_the_city_is_refused_toward_exec() {
        let city = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let asked = elsewhere.path().join("notes.md");
        let err = within_city(city.path(), asked.to_str().unwrap(), "read").unwrap_err();
        assert_eq!(
            (err.code(), err.recovery().contains("`exec`")),
            (&AxCode::GateDenied, true),
            "{err:?}"
        );
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
            (ReadVerdict::RulesUnreadable(unread), "the User has to fix"),
        ] {
            let err = admit("vault/room1/notes.md", "read", &|_| verdict.clone()).unwrap_err();
            assert_eq!(err.code(), &AxCode::GateDenied);
            assert!(err.recovery().contains(says), "{}", err.recovery());
        }
    }
}
