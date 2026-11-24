// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::path::Path;

use crate::alias::WriteTarget;
use crate::error::{StorageError, io_err};
use crate::vfs::Vfs;

use super::landing::{Bits, land};

pub(crate) const HISTORY: &str = "history";
const PACK: &str = "history.pack";
const REFS: &str = "refs";
const SYMBOLIC: &str = "ref:";
const REFLOG: &str = "restore a city bundle";

fn git_err(op: &'static str) -> impl FnOnce(git2::Error) -> StorageError {
    move |err| StorageError::Bundle {
        op,
        detail: err.message().to_owned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Carried {
    pub(crate) packs: u64,
    pub(crate) refs: u64,
}

impl Carried {
    pub(crate) fn of(vfs: &dyn Vfs, bundle: &Path) -> Result<Carried, StorageError> {
        let dir = bundle.join(HISTORY);
        let mut total = Self::default();
        for path in project_dirs(vfs, &dir)? {
            let at = path.join(REFS);
            let refs = vfs
                .read(&at)
                .map_err(io_err("read the project refs", &at))?;
            let lines = refs
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .count();
            total.refs = total
                .refs
                .saturating_add(u64::try_from(lines).map_err(|_| malformed(&at))?);
            total.packs = total
                .packs
                .saturating_add(u64::from(vfs.exists(&path.join(PACK))));
        }
        Ok(total)
    }
}

pub(crate) fn export(vfs: &mut dyn Vfs, city_root: &Path, dest: &Path) -> Result<(), StorageError> {
    for project in crate::ProjectRepository::list(city_root)? {
        let repo =
            git2::Repository::open(project.root()).map_err(git_err("export project history"))?;
        let dir = dest.join(HISTORY).join(project.building().as_str());
        write_project(vfs, dest, &dir, &repo)?;
    }
    Ok(())
}

fn project_dirs(vfs: &dyn Vfs, history: &Path) -> Result<Vec<std::path::PathBuf>, StorageError> {
    let files = match vfs.list(history) {
        Ok(files) => files,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(io_err("list project histories", history)(err)),
    };
    if !files.is_empty() {
        return Err(malformed(history));
    }
    let mut dirs = vfs
        .list_dirs(history)
        .map_err(io_err("list project histories", history))?;
    dirs.sort();
    Ok(dirs)
}

fn write_project(
    vfs: &mut dyn Vfs,
    dest: &Path,
    dir: &Path,
    repo: &git2::Repository,
) -> Result<(), StorageError> {
    let (pack, refs) = pack_of(repo, "export")?;
    vfs.create_dir_all(dir)
        .map_err(io_err("make a bundle directory", dir))?;
    if let Some(bytes) = pack {
        let target = WriteTarget::within("write a history pack", dest, &dir.join(PACK))?;
        land(vfs, target, &bytes, Bits::OfReplaced)?;
    }
    let target = WriteTarget::within("write the history refs", dest, &dir.join(REFS))?;
    land(vfs, target, refs.as_bytes(), Bits::OfReplaced)
}

fn pack_of(
    repo: &git2::Repository,
    op: &'static str,
) -> Result<(Option<Vec<u8>>, String), StorageError> {
    let mut pack = repo.packbuilder().map_err(git_err(op))?;
    let mut walk = repo.revwalk().map_err(git_err(op))?;
    let mut refs = String::new();
    let head = repo.find_reference("HEAD").map_err(git_err(op))?;
    let trunk = head
        .symbolic_target()
        .map_err(git_err(op))?
        .map(str::to_owned);
    let listed = repo.references().map_err(git_err(op))?;
    for reference in std::iter::once(Ok(head)).chain(listed) {
        let reference = reference.map_err(git_err(op))?;
        let name = reference.name().map_err(git_err(op))?;
        if name != "HEAD" && Some(name) != trunk.as_deref() && !name.starts_with("refs/sprawling/")
        {
            continue;
        }
        if let Some(target) = reference.symbolic_target().map_err(git_err(op))? {
            refs.push_str(&format!("{SYMBOLIC}{target} {name}\n"));
        } else if let Some(oid) = reference.target() {
            pack.insert_recursive(oid, None).map_err(git_err(op))?;
            if let Ok(commit) = reference.peel_to_commit() {
                walk.push(commit.id()).map_err(git_err(op))?;
            }
            refs.push_str(&format!("{oid} {name}\n"));
        }
    }
    pack.insert_walk(&mut walk).map_err(git_err(op))?;
    if pack.object_count() == 0 {
        return Ok((None, refs));
    }
    let mut bytes = git2::Buf::new();
    pack.write_buf(&mut bytes).map_err(git_err(op))?;
    Ok((Some(bytes.to_vec()), refs))
}

enum Target {
    Object(git2::Oid),
    Symbolic(String),
}

pub(crate) struct History {
    pack: Option<Vec<u8>>,
    refs: Vec<(String, Target)>,
}

impl History {
    pub(crate) fn read(
        vfs: &dyn Vfs,
        bundle: &Path,
        city_root: &Path,
    ) -> Result<Vec<(crate::ProjectRepository, History)>, StorageError> {
        let mut histories = Vec::new();
        for dir in project_dirs(vfs, &bundle.join(HISTORY))? {
            crate::alias::WriteTarget::within("read bundled project history", bundle, &dir)?;
            let name = dir
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| malformed(&dir))?;
            let addr =
                kernel::Address::parse(name).map_err(|source| StorageError::Draft { source })?;
            let project = crate::ProjectRepository::at(city_root, &addr)?;
            if project
                .root()
                .try_exists()
                .map_err(io_err("inspect the private restore target", project.root()))?
            {
                return Err(StorageError::Bundle {
                    op: "restore project history",
                    detail: format!("{} already exists", project.root().display()),
                });
            }
            histories.push((project, Self::project(vfs, &dir)?));
        }
        Ok(histories)
    }

    fn project(vfs: &dyn Vfs, dir: &Path) -> Result<Self, StorageError> {
        let at = dir.join(REFS);
        let bytes = vfs.read(&at).map_err(io_err("read project refs", &at))?;
        let text = String::from_utf8(bytes).map_err(|_| malformed(&at))?;
        let path = dir.join(PACK);
        let pack = if vfs.exists(&path) {
            Some(
                vfs.read(&path)
                    .map_err(io_err("read project history", &path))?,
            )
        } else {
            None
        };
        let refs = text
            .lines()
            .map(|line| parse(line).ok_or_else(|| malformed(&at)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(History { pack, refs })
    }

    pub(crate) fn land(self, city_root: &Path) -> Result<(), StorageError> {
        let repo = git2::Repository::init(city_root).map_err(git_err("restore"))?;
        if let Some(bytes) = self.pack {
            let odb = repo.odb().map_err(git_err("restore"))?;
            let mut writer = odb.packwriter().map_err(git_err("restore"))?;
            writer
                .write_all(&bytes)
                .map_err(io_err("index a history pack", repo.path()))?;
            writer.commit().map_err(git_err("restore"))?;
        }
        for (name, target) in &self.refs {
            match target {
                Target::Object(oid) => repo.reference(name, *oid, true, REFLOG),
                Target::Symbolic(to) => repo.reference_symbolic(name, to, true, REFLOG),
            }
            .map_err(git_err("restore"))?;
        }
        match repo.head().and_then(|head| head.peel_to_tree()) {
            Ok(tree) => {
                let mut index = repo.index().map_err(git_err("restore"))?;
                index.read_tree(&tree).map_err(git_err("restore"))?;
                index.write().map_err(git_err("restore"))
            }
            Err(err) if err.code() == git2::ErrorCode::UnbornBranch => Ok(()),
            Err(err) => Err(git_err("restore")(err)),
        }
    }
}

fn parse(line: &str) -> Option<(String, Target)> {
    let (target, name) = line.split_once(' ')?;
    let admitted = |ref_name: &str| ref_name == "HEAD" || ref_name.starts_with("refs/");
    let valid = |ref_name: &str| admitted(ref_name) && git2::Reference::is_valid_name(ref_name);
    if !valid(name) {
        return None;
    }
    let target = match target.strip_prefix(SYMBOLIC) {
        Some(to) if valid(to) => Target::Symbolic(to.to_owned()),
        Some(_) => return None,
        None => Target::Object(git2::Oid::from_str(target).ok()?),
    };
    Some((name.to_owned(), target))
}

fn malformed(at: &Path) -> StorageError {
    StorageError::Bundle {
        op: "restore",
        detail: format!("{} holds a line that is not a ref", at.display()),
    }
}
