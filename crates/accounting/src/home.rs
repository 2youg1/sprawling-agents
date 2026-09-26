// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This person's home directory, and what this product keeps under it
//! (sprawling-SPEC.md section 8-70).
//!
//! Three callers derived the same directory before this module: the
//! doctor looking for downloaded components, the installer looking for
//! a place to put the binary, and the first screen looking for
//! somewhere to put a city. Each read `USERPROFILE` and then `HOME`,
//! each wrote its own join, and a fourth caller — the person-level
//! configuration layer — was about to write a fifth.
//!
//! A home is a value here: `detect` reads the environment once, and
//! every path below is derived from the directory it found. What lives
//! under the home directory is this machine's, never a city's: a city
//! carried to another machine must not carry this machine's components
//! or this person's configuration with it.

use std::path::{Path, PathBuf};

use kernel::{AxCode, AxError};

/// Components this machine downloaded: one directory per component,
/// named after it.
const COMPONENTS_DIR: &str = "components";

/// What this person declares for every city they run. Lower case, and
/// deliberately not the city's own `CONFIG.toml`: the two are different
/// layers, and a shared name would invite one reader to load the other.
const PERSON_CONFIG_FILE: &str = "config.toml";

/// Where this product puts the cities it makes for a person who named
/// no location. Outside the dot directory on purpose: a person opens
/// this one, edits the documents in it and copies it to another
/// machine, while everything under the dot directory belongs to this
/// machine alone.
const CITIES_DIR: &str = "sprawling";

/// The directory name of a city nobody named.
///
/// One name for both placements: `bin::firstrun` decides between the
/// binary's own directory and the home directory, and a second
/// spelling there would let the two answers drift apart.
pub(crate) const CITY_DIR: &str = "city";

/// What this machine looks like when neither variable is set.
///
/// `Home::detect` refuses with this sentence and the doctor reports it
/// for `Absence::NoHome`, so the person reading the refusal and the
/// person reading the report read one fact.
pub(crate) const NO_HOME: &str = "neither USERPROFILE nor HOME is set";

/// This person's home directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    /// This person's home directory, as the environment states it.
    ///
    /// `USERPROFILE` first and `HOME` second: Windows sets the first,
    /// every other platform sets the second, and a Windows shell that
    /// sets both means the first.
    ///
    /// # Errors
    /// Returns `PathNotFound` when neither variable is set, which is
    /// what a service account or a stripped environment looks like.
    pub fn detect() -> Result<Home, AxError> {
        let root = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
        match root {
            Some(root) => Ok(Home {
                root: PathBuf::from(root),
            }),
            None => Err(AxError::failure(
                AxCode::PathNotFound,
                "find this person's home directory",
                NO_HOME,
            )
            .with_recovery("set HOME to the directory this person's files live in")),
        }
    }

    /// A home at a named directory, for tests that compare paths
    /// rather than read this machine's environment.
    #[cfg(test)]
    pub(crate) fn at(root: &Path) -> Home {
        Home {
            root: root.to_path_buf(),
        }
    }

    /// The home directory itself.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Where this machine keeps the components it downloaded.
    ///
    /// Under the home directory rather than inside any city, because a
    /// component is built for this operating system and this processor
    /// while a city is copied between machines.
    #[must_use]
    pub fn components(&self) -> PathBuf {
        self.product_dir().join(COMPONENTS_DIR)
    }

    /// Where this person declares what every city they run inherits.
    ///
    /// This module states the path only. Reading the file, and where
    /// its values sit among the city, building and resident layers, is
    /// the configuration ladder's.
    #[must_use]
    pub fn person_config(&self) -> PathBuf {
        self.product_dir().join(PERSON_CONFIG_FILE)
    }

    /// Where a city goes when the person named none and the binary's
    /// own directory will not take one.
    ///
    /// Not under the dot directory: a city is the person's own work
    /// rather than this machine's state, and a city they cannot see in
    /// their file manager is a city they cannot carry away.
    #[must_use]
    pub fn default_city(&self) -> PathBuf {
        self.root.join(CITIES_DIR).join(CITY_DIR)
    }

    /// This product's own directory under the home directory.
    ///
    /// The same dot name a city reserves for what governs it, because
    /// it is the one directory name this product owns; here it holds
    /// what belongs to the person rather than to a city.
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

    /// A city is the person's own work, so it lands where they can see
    /// it rather than under the directory this machine keeps its own
    /// state in.
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
