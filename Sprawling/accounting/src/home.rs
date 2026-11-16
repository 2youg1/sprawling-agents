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

    fn product_dir(&self) -> PathBuf {
        self.root.join(kernel::RESERVED_PREFIX)
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
