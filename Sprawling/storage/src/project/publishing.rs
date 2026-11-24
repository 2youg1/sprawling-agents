// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::GitOid;

use super::ProjectRepository;
use super::source::failed;
use crate::StorageError;

impl ProjectRepository {
    pub fn publish(
        &self,
        reviewed: GitOid,
        destination: &Path,
        base: &str,
        branch: &str,
    ) -> Result<String, StorageError> {
        let refused = failed("publish reviewed project changes", destination);
        let reject = |reason: &str| refused(git2::Error::from_str(reason));
        let base_ref = format!("refs/heads/{base}");
        let target = format!("refs/heads/{branch}");
        if base == branch
            || !git2::Reference::is_valid_name(&base_ref)
            || !git2::Reference::is_valid_name(&target)
        {
            return Err(reject("select two distinct valid local branch names"));
        }
        let private = git2::Repository::open(self.root()).map_err(&refused)?;
        let oid = git2::Oid::from_str(&reviewed.to_string()).map_err(&refused)?;
        if private
            .refname_to_id(&format!("refs/sprawling/reviewed/{oid}"))
            .map_err(&refused)?
            != oid
        {
            return Err(reject("the review receipt does not name this merge"));
        }
        let receipt = private.find_commit(oid).map_err(&refused)?;
        let dest = git2::Repository::open_ext(
            destination,
            git2::RepositoryOpenFlags::NO_SEARCH,
            std::iter::empty::<&Path>(),
        )
        .map_err(&refused)?;
        let native = dest.refname_to_id(&base_ref).map_err(&refused)?;
        let mut registered = false;
        for reference in private
            .references_glob("refs/sprawling/source/*")
            .map_err(&refused)?
        {
            let reference = reference.map_err(&refused)?;
            let name = reference.name().map_err(&refused)?;
            let (prefix, original) = name
                .rsplit_once('/')
                .ok_or_else(|| reject("invalid source binding"))?;
            if prefix != "refs/sprawling/source" {
                continue;
            }
            let original = git2::Oid::from_str(original).map_err(&refused)?;
            let baseline = reference.peel_to_commit().map_err(&refused)?;
            if (native == original
                || (dest.odb().map_err(&refused)?.exists(original)
                    && dest
                        .graph_descendant_of(native, original)
                        .map_err(&refused)?))
                && private
                    .graph_descendant_of(oid, baseline.id())
                    .map_err(&refused)?
            {
                registered = true;
                break;
            }
        }
        if !registered {
            return Err(reject(
                "the reviewed merge and destination branch do not descend from the registered source",
            ));
        }
        let native_parent = dest.find_commit(native).map_err(&refused)?;
        let objects = dest.commondir().join("objects");
        private
            .odb()
            .and_then(|odb| {
                odb.add_disk_alternate(objects.to_str().ok_or_else(|| {
                    git2::Error::from_str("destination object directory is not Unicode")
                })?)
            })
            .map_err(&refused)?;
        let subtree = |commit: &git2::Commit<'_>| -> Result<git2::Tree<'_>, git2::Error> {
            let wrapped = commit.tree()?;
            wrapped
                .get_path(Path::new(self.building.as_str()))?
                .to_object(&private)?
                .peel_to_tree()
        };
        let reviewed_base = receipt.parent(0).map_err(&refused)?;
        let original = subtree(&reviewed_base).map_err(&refused)?;
        let updated = contribution(&private, &original, &subtree(&receipt).map_err(&refused)?)
            .map_err(&refused)?;
        let native_tree = private
            .find_tree(native_parent.tree_id())
            .map_err(&refused)?;
        let mut merged = private
            .merge_trees(&original, &native_tree, &updated, None)
            .map_err(&refused)?;
        if merged.has_conflicts() {
            return Err(reject(
                "the published tree conflicts with the registered source tree",
            ));
        }
        let id = merged.write_tree_to(&private).map_err(&refused)?;
        transfer(&private, &dest, id).map_err(&refused)?;
        let published = dest
            .commit(
                None,
                &receipt.author(),
                &receipt.committer(),
                receipt.message().map_err(&refused)?,
                &dest.find_tree(id).map_err(&refused)?,
                &[&native_parent],
            )
            .map_err(&refused)?;
        let mut transaction = dest.transaction().map_err(&refused)?;
        transaction.lock_ref(&base_ref).map_err(&refused)?;
        transaction.lock_ref(&target).map_err(&refused)?;
        if dest.refname_to_id(&base_ref).map_err(&refused)? != native {
            return Err(reject("the source branch advanced after registration"));
        }
        match dest.find_reference(&target) {
            Ok(current) if current.target() == Some(published) => return Ok(published.to_string()),
            Ok(_) => {
                return Err(reject(
                    "the contribution branch already exists; select a new branch",
                ));
            }
            Err(err) if err.code() == git2::ErrorCode::NotFound => {}
            Err(err) => return Err(refused(err)),
        }
        transaction
            .set_target(
                &target,
                published,
                Some(&receipt.committer()),
                "publish reviewed project changes",
            )
            .and_then(|()| transaction.commit())
            .map_err(&refused)?;
        Ok(published.to_string())
    }
}

fn contribution<'repo>(
    repo: &'repo git2::Repository,
    baseline: &git2::Tree<'repo>,
    reviewed: &git2::Tree<'repo>,
) -> Result<git2::Tree<'repo>, git2::Error> {
    let changes = repo.diff_tree_to_tree(Some(baseline), Some(reviewed), None)?;
    let records: Vec<_> = changes
        .deltas()
        .filter_map(|delta| {
            delta
                .new_file()
                .path()
                .filter(|path| private_record(path))
                .map(Path::to_owned)
        })
        .collect();
    if records.is_empty() {
        return repo.find_tree(reviewed.id());
    }
    let mut original = git2::Index::new()?;
    original.read_tree(baseline)?;
    let mut code = git2::Index::new()?;
    code.read_tree(reviewed)?;
    for path in records {
        if let Some(entry) = original.get_path(&path, 0) {
            code.add(&entry)?;
        } else if code.get_path(&path, 0).is_some() {
            code.remove_path(&path)?;
        }
    }
    repo.find_tree(code.write_tree_to(repo)?)
}

fn private_record(path: &Path) -> bool {
    !crate::reserved::outside_reserved(path)
        || path.starts_with(kernel::layout::ARCHIVE_DIR)
        || path
            .file_name()
            .is_some_and(|name| name == kernel::layout::HANDOFF_FILE)
}

fn transfer(
    source: &git2::Repository,
    destination: &git2::Repository,
    root: git2::Oid,
) -> Result<(), git2::Error> {
    let src = source.odb()?;
    let dest = destination.odb()?;
    let mut pending = vec![(root, false)];
    while let Some((id, expanded)) = pending.pop() {
        if dest.exists(id) {
            continue;
        }
        let object = src.read(id)?;
        if !expanded && object.kind() == git2::ObjectType::Tree {
            pending.push((id, true));
            let tree = source.find_tree(id)?;
            pending.extend(
                tree.iter()
                    .filter(|entry| entry.kind() != Some(git2::ObjectType::Commit))
                    .map(|entry| (entry.id(), false)),
            );
            continue;
        }
        dest.write(object.kind(), object.data())?;
    }
    Ok(())
}
