// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

const COMPONENTS_DIR: &str = "components";

const PERSON_CONFIG_FILE: &str = "config.toml";

const CITIES_DIR: &str = "sprawling";

pub const CITY_DIR: &str = "city";

const BACKUPS_DIR: &str = "backups";

pub const NO_HOME: &str = "neither USERPROFILE nor HOME is set";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    pub fn detect() -> Result<Home, AxError> {
        let root = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
        match root {
            Some(root) => Ok(Home::at(root)),
            None => Err(AxError::failure(
                AxCode::PathNotFound,
                "find the User's home directory",
                NO_HOME,
            )
            .with_recovery("set HOME to the directory the User's files live in")),
        }
    }

    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Home {
        Home { root: root.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn components(&self) -> PathBuf {
        self.product_dir().join(COMPONENTS_DIR)
    }

    #[must_use]
    pub fn privacy_history(&self) -> PathBuf {
        self.product_dir().join("privacy").join("changes.jsonl")
    }

    #[must_use]
    pub fn person_config(&self) -> PathBuf {
        self.product_dir().join(PERSON_CONFIG_FILE)
    }

    #[must_use]
    pub fn backups(&self) -> PathBuf {
        self.product_dir().join(BACKUPS_DIR)
    }

    #[must_use]
    pub fn default_city(&self) -> PathBuf {
        self.root.join(CITIES_DIR).join(CITY_DIR)
    }

    pub fn workspace(&self, selected: &Path) -> Result<PathBuf, AxError> {
        let path = physical(selected)?;
        let stores = physical(&self.product_dir().join("w"))?;
        if path.parent() == Some(stores.as_path())
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.len() == 32
                        && name
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
        {
            return Ok(path);
        }
        if stores.starts_with(&path) || path.starts_with(&stores) {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "select a workspace",
                path.display().to_string(),
            )
            .with_recovery("select a project folder outside the private workspace store"));
        }
        let id = kernel::B3Hash::digest(path.as_os_str().as_encoded_bytes());
        let private = stores.join(id.to_string().chars().take(32).collect::<String>());
        if physical(&private)? != private {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "resolve private workspace storage",
                private.display().to_string(),
            )
            .with_recovery(
                "remove the redirected private workspace entry and open the workspace again",
            ));
        }
        Ok(private)
    }

    fn product_dir(&self) -> PathBuf {
        self.root.join(kernel::RESERVED_PREFIX)
    }
}

fn physical(path: &Path) -> Result<PathBuf, AxError> {
    let refused = |err: std::io::Error| {
        AxError::failure(
            AxCode::PathNotFound,
            "resolve the workspace location",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("choose a directory whose parent exists and is readable")
    };
    let absolute = std::path::absolute(path).map_err(refused)?;
    let mut missing = Vec::new();
    for ancestor in absolute.ancestors() {
        match std::fs::canonicalize(ancestor) {
            Ok(mut found) => {
                found.extend(missing.into_iter().rev());
                return Ok(found);
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if let Some(name) = ancestor.file_name() {
                    missing.push(name);
                } else {
                    return Err(refused(err));
                }
            }
            Err(err) => return Err(refused(err)),
        }
    }
    Err(refused(std::io::ErrorKind::NotFound.into()))
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

    fn home() -> Home {
        Home::at(Path::new("dwelling"))
    }

    #[test]
    fn what_this_machine_holds_sits_beside_what_this_person_declares() {
        let product = Path::new("dwelling").join(kernel::RESERVED_PREFIX);
        assert_eq!(home().components(), product.join(COMPONENTS_DIR));
        assert_eq!(home().person_config(), product.join(PERSON_CONFIG_FILE));
    }

    #[test]
    fn a_city_nobody_named_lands_where_the_person_can_see_it() {
        let city = home().default_city();
        assert_eq!(city, Path::new("dwelling").join(CITIES_DIR).join(CITY_DIR));
        assert!(
            !city.starts_with(Path::new("dwelling").join(kernel::RESERVED_PREFIX)),
            "a city is not this machine's state: {}",
            city.display()
        );
    }
}
