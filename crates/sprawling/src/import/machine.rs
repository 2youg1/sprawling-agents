// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two files on this machine another harness already wrote, and
//! where they are (sprawling-SPEC.md section 8-71).
//!
//! **One direction, once.** This reads what is on the disk and turns it
//! into rows a person can act on. It never writes into either file, and
//! it never watches them: a person who changes a provider in their
//! other tool has changed that tool, and a city that followed would be
//! a second reader of somebody else's settings.
//!
//! An absent file is the ordinary case rather than a failure - most
//! machines have one of these harnesses or neither. A file that is
//! there and cannot be read is a failure, because the person asked for
//! an import and would otherwise be told that a harness they use is
//! not configured.

use std::path::PathBuf;

use kernel::{AxCode, AxError};

use super::provider::Imported;
use super::{codex, pi};
use crate::home::Home;

/// Codex's directory under the home directory, and the file in it.
const CODEX_DIR: &str = ".codex";
const CODEX_FILE: &str = "config.toml";

/// pi's directory under the home directory, and the file in it.
const PI_DIR: &str = ".pi";
const PI_AGENT_DIR: &str = "agent";
const PI_FILE: &str = "models.json";

/// What every harness on this machine states about providers.
///
/// The two readers are asked in a fixed order and their findings are
/// concatenated, so a person reading the offer twice reads it in the
/// same order both times.
///
/// # Errors
/// Propagates a file that is present and cannot be read, and one that
/// is present and does not parse.
pub(crate) fn scan(home: &Home) -> Result<Imported, AxError> {
    let mut found = Imported::default();
    if let Some(text) = held(codex_config(home))? {
        found.absorb(codex::read(&text)?);
    }
    if let Some(text) = held(pi_models(home))? {
        found.absorb(pi::read(&text)?);
    }
    Ok(found)
}

/// Where Codex keeps what this city reads.
fn codex_config(home: &Home) -> PathBuf {
    home.path().join(CODEX_DIR).join(CODEX_FILE)
}

/// Where pi keeps what this city reads.
fn pi_models(home: &Home) -> PathBuf {
    home.path().join(PI_DIR).join(PI_AGENT_DIR).join(PI_FILE)
}

/// One file's text, or nothing when this machine does not have it.
///
/// # Errors
/// Propagates every reason a present file could not be read. The one
/// case answered rather than propagated is absence, which is what a
/// machine without that harness looks like.
fn held(path: PathBuf) -> Result<Option<String>, AxError> {
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read the providers another harness is configured with",
            format!("{}: {err}", path.display()),
        )
        .with_recovery(
            "make the file readable by this account, or attach the endpoint here by hand",
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

    /// A machine with neither harness imports nothing and says so by
    /// answering, rather than by refusing.
    #[test]
    fn a_machine_with_neither_harness_imports_nothing() {
        let empty = tempfile::tempdir().unwrap();
        let home = Home::at(empty.path());
        assert_eq!(scan(&home).unwrap(), Imported::default());
    }

    /// Both harnesses at once, each read from its own place, and the
    /// rows carry which file they came from.
    #[test]
    fn both_harnesses_are_read_and_each_row_says_where_it_came_from() {
        let machine = tempfile::tempdir().unwrap();
        let home = Home::at(machine.path());
        std::fs::create_dir_all(machine.path().join(CODEX_DIR)).unwrap();
        std::fs::write(
            codex_config(&home),
            "[model_providers.relay]\nbase_url = \"https://relay.example.com/v1\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(machine.path().join(PI_DIR).join(PI_AGENT_DIR)).unwrap();
        std::fs::write(
            pi_models(&home),
            r#"{"providers":{"ollama":{"baseUrl":"http://localhost:11434/v1"}}}"#,
        )
        .unwrap();
        let found = scan(&home).unwrap();
        let named: Vec<(&str, &str)> = found
            .providers
            .iter()
            .map(|row| (row.harness.as_str(), row.name.as_str()))
            .collect();
        assert_eq!(named, vec![("codex", "relay"), ("pi", "ollama")]);
    }

    /// A file that is there and does not parse is a refusal, not an
    /// empty import: a person told "no providers found" would go and
    /// look for a file that is sitting right there.
    #[test]
    fn a_file_that_does_not_parse_refuses() {
        let machine = tempfile::tempdir().unwrap();
        let home = Home::at(machine.path());
        std::fs::create_dir_all(machine.path().join(CODEX_DIR)).unwrap();
        std::fs::write(codex_config(&home), "[model_providers.p").unwrap();
        assert!(scan(&home).is_err());
    }
}
