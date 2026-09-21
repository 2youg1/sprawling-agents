// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Codex's `[model_providers.*]` table, read once
//! (sprawling-SPEC.md section 8-71).
//!
//! # The grammar, and how it was settled
//!
//! Every key below was checked against the Codex binary's own parser by
//! giving it a configuration with that key set to a value of the wrong
//! type and reading which key it named in the refusal. That is the
//! authority for this file: the program that reads it.
//!
//! - `[model_providers.<id>]` — one table per provider, keyed by the id
//!   `model_provider` selects it with.
//! - `base_url` (string), `env_key` (string), `wire_api` (enum),
//!   `name` (string), `query_params` / `http_headers` /
//!   `env_http_headers` (maps), `requires_openai_auth` (boolean),
//!   `stream_idle_timeout_ms` (unsigned).
//! - `wire_api` admits `responses` and nothing else in the build
//!   measured; `chat` is refused with "no longer supported". A file
//!   written before that change still says `chat`, and a file on disk
//!   is what this module reads, so both spellings are mapped.
//!
//! Unknown keys are ignored rather than refused, which is what Codex
//! itself does with them: this city reads four facts out of a file
//! whose other settings are none of its business.

use toml::{Table, Value};

use super::provider::{Credential, Harness, Imported, Spoken, Stated};

/// The table every provider entry sits under.
const PROVIDERS: &str = "model_providers";

/// Codex's own spelling of the OpenAI chat completions wire, retired
/// upstream and still present in files written before it was.
const CHAT: &str = "chat";

/// What one Codex configuration file states about providers.
///
/// # Errors
/// Propagates a file that is not TOML. A configuration this city cannot
/// parse is not one to read four facts out of by guessing, and the
/// person is holding a file their other tool also refuses.
pub(crate) fn read(text: &str) -> Result<Imported, kernel::AxError> {
    let document: Table = text.parse().map_err(|err: toml::de::Error| {
        kernel::AxError::failure(
            kernel::AxCode::ConfigInvalid,
            "read the providers another harness is configured with",
            format!("~/.codex/config.toml: {err}"),
        )
        .with_recovery(
            "fix the file where codex reads it, or attach the endpoint here by hand; nothing \
             was imported",
        )
    })?;
    let mut found = Imported::default();
    let Some(Value::Table(providers)) = document.get(PROVIDERS) else {
        return Ok(found);
    };
    for (id, entry) in providers {
        match entry {
            Value::Table(settings) => found.take(Harness::Codex, stated(id, settings)),
            Value::String(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::Boolean(_)
            | Value::Datetime(_)
            | Value::Array(_) => found.malformed(Harness::Codex, id.clone()),
        }
    }
    Ok(found)
}

/// One entry, in this city's terms.
///
/// The table's key is the name carried, because that is the identity
/// Codex itself selects a provider by; the entry's own `name` is a
/// label for its screen and would be a second answer to "which provider
/// is this".
fn stated(id: &str, settings: &Table) -> Stated {
    Stated {
        named: id.to_owned(),
        base_url: text(settings, "base_url"),
        wire: wire(settings),
        credential: credential(settings),
        // Codex states no model list per provider, so this city admits
        // whatever the endpoint serves rather than inventing a list.
        models: Vec::new(),
    }
}

/// Which wire the entry names.
fn wire(settings: &Table) -> Spoken {
    match text(settings, "wire_api") {
        None => Spoken::Unstated,
        Some(stated) => {
            if stated == CHAT {
                Spoken::Speaks(kernel::DialectKind::OpenAi)
            } else {
                Spoken::Foreign { stated }
            }
        }
    }
}

/// Where the entry keeps its credential.
///
/// `experimental_bearer_token` holds a token in the file itself, so it
/// is reported as present and never read: the value belongs in the
/// vault, and copying it here would put it in a second place.
fn credential(settings: &Table) -> Credential {
    match text(settings, "env_key") {
        Some(variable) => Credential::Environment { variable },
        None => match settings.get("experimental_bearer_token") {
            Some(_) => Credential::NotCarried,
            None => Credential::Unstated,
        },
    }
}

/// One string-valued key, or nothing when it is absent or is not a
/// string.
fn text(settings: &Table, key: &str) -> Option<String> {
    match settings.get(key) {
        Some(Value::String(found)) => Some(found.clone()),
        Some(_) | None => None,
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
    use crate::import::provider::Unimportable;

    /// The file this machine's own Codex writes has tables that are not
    /// providers at all, and the reader has to walk past every one of
    /// them.
    const SURROUNDED: &str = r#"
notify = ["a", "b"]

[tui]
theme = "zenburn"

[model_providers.relay]
name = "Relay"
base_url = "https://relay.example.com/v1"
env_key = "RELAY_API_KEY"
wire_api = "responses"

[mcp_servers.node_repl]
command = "node"
"#;

    #[test]
    fn one_entry_becomes_one_endpoint_with_its_key_for_a_name() {
        let found = read(SURROUNDED).unwrap();
        assert!(found.unusable.is_empty());
        assert_eq!(found.providers.len(), 1);
        let row = &found.providers[0];
        assert_eq!(row.name.as_str(), "relay");
        assert_eq!(row.base_url, "https://relay.example.com/v1");
        assert_eq!(
            row.credential,
            Credential::Environment {
                variable: "RELAY_API_KEY".to_owned()
            }
        );
        assert!(row.models.is_empty());
    }

    /// The wire Codex admits today is one this city has no dialect for,
    /// so it is carried by name instead of guessed at.
    #[test]
    fn the_responses_wire_is_named_rather_than_guessed() {
        let found = read(SURROUNDED).unwrap();
        assert_eq!(
            found.providers[0].wire,
            Spoken::Foreign {
                stated: "responses".to_owned()
            }
        );
    }

    #[test]
    fn the_retired_chat_wire_is_the_openai_dialect() {
        let found = read("[model_providers.p]\nbase_url = \"u\"\nwire_api = \"chat\"\n").unwrap();
        assert_eq!(
            found.providers[0].wire,
            Spoken::Speaks(kernel::DialectKind::OpenAi)
        );
    }

    /// An entry with nowhere to call is reported by name, because a
    /// person who configured it has to be told why it was left out.
    #[test]
    fn an_entry_with_no_base_url_is_reported_rather_than_dropped() {
        let found = read("[model_providers.p]\nenv_key = \"K\"\n").unwrap();
        assert!(found.providers.is_empty());
        assert_eq!(found.unusable.len(), 1);
        assert_eq!(found.unusable[0].named, "p");
        assert_eq!(found.unusable[0].because, Unimportable::NoAddress);
    }

    /// A token written into the other harness's file is reported as
    /// present and never copied.
    #[test]
    fn a_token_in_their_file_is_reported_and_not_carried() {
        let found = read(
            "[model_providers.p]\nbase_url = \"u\"\nexperimental_bearer_token = \"sk-secret\"\n",
        )
        .unwrap();
        assert_eq!(found.providers[0].credential, Credential::NotCarried);
    }

    #[test]
    fn a_file_that_is_not_toml_refuses_rather_than_importing_half_of_it() {
        assert!(read("[model_providers.p").is_err());
    }

    #[test]
    fn a_file_with_no_provider_table_imports_nothing() {
        assert_eq!(read("[tui]\ntheme = \"z\"\n").unwrap(), Imported::default());
    }
}
