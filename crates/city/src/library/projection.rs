// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A disposable, read-only projection of one skill manifest. Auditors
//! and script runners need real paths; the CAS remains the authority,
//! and every projected byte is read through the verifying callback.

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError, B3Hash, SkillItem};

use super::reading;

/// Rebuilds `.sprawling/library/.objects/<manifest digest>` from CAS.
/// The returned directory is a projection, never a shelf or an authority.
pub fn materialize(
    city_root: &Path,
    digest: &B3Hash,
    read: &dyn Fn(&B3Hash) -> Result<Vec<u8>, AxError>,
) -> Result<PathBuf, AxError> {
    let manifest = reading::read_manifest(read, digest)?;
    let objects = kernel::layout::CityLayout::new(city_root)
        .library()
        .join(".objects");
    let target = objects.join(digest.to_string());
    let staged = objects.join(format!(".{digest}.part"));
    if staged.exists() {
        std::fs::remove_dir_all(&staged).map_err(|err| projection_io(&staged, err))?;
    }
    if target.exists() {
        std::fs::remove_dir_all(&target).map_err(|err| projection_io(&target, err))?;
    }
    std::fs::create_dir_all(&staged).map_err(|err| projection_io(&staged, err))?;
    let mut files = Vec::new();
    for item in manifest.items() {
        let path = item
            .path()
            .split('/')
            .fold(staged.clone(), |at, segment| at.join(segment));
        match item {
            SkillItem::Directory { .. } => {
                std::fs::create_dir_all(&path).map_err(|err| projection_io(&path, err))?;
            }
            SkillItem::File { digest, .. } => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|err| projection_io(parent, err))?;
                }
                let bytes = read(digest)?;
                std::fs::write(&path, bytes).map_err(|err| projection_io(&path, err))?;
                files.push(item.path().to_owned());
            }
        }
    }
    std::fs::rename(&staged, &target).map_err(|err| projection_io(&target, err))?;
    for relative in files {
        let file = target.join(relative);
        let mut permissions = std::fs::metadata(&file)
            .map_err(|err| projection_io(&file, err))?
            .permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&file, permissions).map_err(|err| projection_io(&file, err))?;
    }
    Ok(target)
}

fn projection_io(path: &Path, err: std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "materialize a skill projection",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("make the city's library writable and rebuild the projection from CAS")
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
    use kernel::SkillManifest;

    #[test]
    fn a_projection_contains_the_verified_files_and_is_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let document = b"# Review\n".to_vec();
        let script = b"echo review\n".to_vec();
        let document_digest = B3Hash::digest(&document);
        let script_digest = B3Hash::digest(&script);
        let manifest = SkillManifest::new(vec![
            SkillItem::Directory {
                path: "scripts".to_owned(),
            },
            SkillItem::File {
                path: "scripts/check.sh".to_owned(),
                digest: script_digest,
            },
            SkillItem::File {
                path: "SKILL.md".to_owned(),
                digest: document_digest,
            },
        ])
        .unwrap();
        let manifest_bytes = manifest.encode().unwrap();
        let digest = B3Hash::digest(&manifest_bytes);
        let read = |hash: &B3Hash| {
            if hash == &digest {
                Ok(manifest_bytes.clone())
            } else if hash == &document_digest {
                Ok(document.clone())
            } else if hash == &script_digest {
                Ok(script.clone())
            } else {
                Err(
                    AxError::failure(AxCode::PathNotFound, "read a test object", hash.to_string())
                        .with_recovery("name a stored object"),
                )
            }
        };
        let projected = materialize(dir.path(), &digest, &read).unwrap();
        assert_eq!(std::fs::read(projected.join("SKILL.md")).unwrap(), document);
        assert_eq!(
            std::fs::read(projected.join("scripts/check.sh")).unwrap(),
            script
        );
        assert!(
            std::fs::metadata(projected.join("SKILL.md"))
                .unwrap()
                .permissions()
                .readonly()
        );
        assert_eq!(crate::skill_digest(&projected).unwrap(), digest);
    }
}
