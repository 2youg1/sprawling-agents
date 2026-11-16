// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError, ReadVerdict};
use same_file::Handle;

pub type ReadBound = Arc<dyn Fn(&Address) -> ReadVerdict + Send + Sync>;

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

fn spelled_under(root: &Path, real: &Path) -> Option<String> {
    real.strip_prefix(root)
        .ok()?
        .iter()
        .map(std::ffi::OsStr::to_str)
        .collect::<Option<Vec<&str>>>()
        .map(|segments| segments.join("/"))
}

pub(crate) struct CityRoot {
    written: PathBuf,
    real: PathBuf,
}

impl CityRoot {
    pub(crate) fn anchor(written: &Path, action: &'static str) -> Result<CityRoot, AxError> {
        let real = real_location(written, action, "the city root")?.into_path();
        Ok(CityRoot {
            written: written.to_path_buf(),
            real,
        })
    }

    pub(crate) fn written(&self) -> &Path {
        &self.written
    }

    pub(crate) fn join(&self, addr: &Address) -> PathBuf {
        addr.as_str()
            .split('/')
            .fold(self.written.clone(), |path, segment| path.join(segment))
    }

    pub(crate) fn judged_location(
        &self,
        addr: &Address,
        real: &Path,
        action: &'static str,
        bound: &dyn Fn(&Address) -> ReadVerdict,
    ) -> Result<Address, AxError> {
        let inside = spelled_under(&self.real, real).ok_or_else(|| {
            AxError::failure(
                AxCode::GateDenied,
                action,
                format!("{} leads out of the city through a link", addr.as_str()),
            )
            .with_recovery("name a path whose every link stays inside the city")
        })?;

        if inside == addr.as_str() {
            return Ok(addr.clone());
        }
        admit(&inside, action, bound)
    }
}

pub(crate) struct Judged {
    pub location: Located,
    pub addr: Address,
}

pub(crate) fn land(
    city: &CityRoot,
    addr: &Address,
    action: &'static str,
    bound: &dyn Fn(&Address) -> ReadVerdict,
) -> Result<Judged, AxError> {
    let located = real_location(&city.join(addr), action, addr.as_str())?;
    let landed = city.judged_location(addr, located.path(), action, bound)?;
    Ok(Judged {
        location: located,
        addr: landed,
    })
}

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

pub(crate) enum Located {
    Present(PathBuf),

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

fn no_entry(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(err) if err.kind() == ErrorKind::NotFound)
}

pub(crate) enum Walked {
    Directory(PathBuf),

    File(PathBuf),

    Passed,
}

pub(crate) fn walked(
    city: &CityRoot,
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
    match Address::parse(rel).and_then(|addr| land(city, &addr, "search", bound)) {
        Ok(judged) => match judged.location {
            Located::Present(real) if real.is_file() => Ok(Walked::File(real)),
            Located::Present(_) | Located::Absent(_) => Ok(Walked::Passed),
        },
        Err(refused) if refused.code() == &AxCode::GateDenied => Ok(Walked::Passed),
        Err(unresolved) => Err(unresolved),
    }
}

#[cfg(all(test, windows))]
#[expect(clippy::unwrap_used, reason = "test fixture")]
#[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
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
