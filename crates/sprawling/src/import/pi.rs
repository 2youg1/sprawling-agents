// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! pi's `providers` object, read once (sprawling-SPEC.md section 8-71).
//!
//! # The grammar, and where it comes from
//!
//! pi ships its own documentation of this file beside the program, and
//! that document is the authority for every name below.
//!
//! - `providers.<id>` — one object per provider, keyed by the id.
//! - `baseUrl`, `api`, `apiKey`, `headers`, `authHeader`, `models`,
//!   `modelOverrides` at the provider level.
//! - `api` is one of `openai-completions`, `openai-responses`,
//!   `anthropic-messages`, `google-generative-ai`. The first is this
//!   city's OpenAI dialect and the third is its Anthropic one; the
//!   other two are wires this city has no dialect for.
//! - `models[]` carries `id`, and optionally `contextWindow` and
//!   `maxTokens`, which are the two figures a model list does not
//!   return.
//! - `apiKey` resolves three ways: a leading `!` runs the rest as a
//!   command, a leading `$` names an environment variable, and `$$` and
//!   `$!` escape those two into a literal.
//!
//! **A model-level `api` is not read.** One endpoint here answers in
//! one dialect, so a file that gives its models different wires states
//! something this city's endpoint cannot hold, and the person chooses
//! rather than being given the first model's answer for all of them.

use serde_json::{Map, Value};

use super::provider::{Credential, Harness, Imported, ImportedModel, Spoken, Stated};

/// The object every provider entry sits under.
const PROVIDERS: &str = "providers";

/// pi's own spelling of the OpenAI chat completions wire.
const COMPLETIONS: &str = "openai-completions";

/// pi's own spelling of the Anthropic messages wire.
const MESSAGES: &str = "anthropic-messages";

/// What one pi model file states about providers.
///
/// # Errors
/// Propagates a file that is not JSON, for the reason the Codex reader
/// gives: a file this city cannot parse is one the person's other tool
/// also refuses, and guessing at its contents would import settings
/// nobody wrote.
pub(crate) fn read(text: &str) -> Result<Imported, kernel::AxError> {
    let document: Value = serde_json::from_str(text).map_err(|err| {
        kernel::AxError::failure(
            kernel::AxCode::ConfigInvalid,
            "read the providers another harness is configured with",
            format!("~/.pi/agent/models.json: {err}"),
        )
        .with_recovery(
            "fix the file where pi reads it, or attach the endpoint here by hand; nothing was \
             imported",
        )
    })?;
    let mut found = Imported::default();
    let Some(Value::Object(providers)) = document.get(PROVIDERS) else {
        return Ok(found);
    };
    for (id, entry) in providers {
        match entry {
            Value::Object(settings) => found.take(Harness::Pi, stated(id, settings)),
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::String(_)
            | Value::Array(_) => found.malformed(Harness::Pi, id.clone()),
        }
    }
    Ok(found)
}

/// One entry, in this city's terms.
fn stated(id: &str, settings: &Map<String, Value>) -> Stated {
    Stated {
        named: id.to_owned(),
        base_url: text(settings, "baseUrl"),
        wire: wire(settings),
        credential: credential(settings),
        models: models(settings),
    }
}

/// Which wire the entry names.
fn wire(settings: &Map<String, Value>) -> Spoken {
    match text(settings, "api") {
        None => Spoken::Unstated,
        Some(stated) => {
            if stated == COMPLETIONS {
                Spoken::Speaks(kernel::DialectKind::OpenAi)
            } else if stated == MESSAGES {
                Spoken::Speaks(kernel::DialectKind::Anthropic)
            } else {
                Spoken::Foreign { stated }
            }
        }
    }
}

/// Where the entry keeps its credential.
///
/// The three prefixes are pi's own resolution rules, and the two
/// escapes are read before them: `$$` and `$!` are how that file spells
/// a literal that begins with the character a rule would otherwise
/// claim. A literal is never carried out of the file it sits in.
fn credential(settings: &Map<String, Value>) -> Credential {
    let Some(value) = text(settings, "apiKey") else {
        return Credential::Unstated;
    };
    if value.starts_with("$$") || value.starts_with("$!") {
        return Credential::NotCarried;
    }
    if let Some(line) = value.strip_prefix('!') {
        return Credential::Command {
            line: line.to_owned(),
        };
    }
    match variable(&value) {
        Some(variable) => Credential::Environment { variable },
        None => Credential::NotCarried,
    }
}

/// The one variable a value names, when the whole value is that
/// reference.
///
/// `$FOO` and `${FOO}` are one variable; `${A}_${B}` is a template this
/// city does not resolve, and reporting it as a variable would name a
/// variable that holds only part of the key.
fn variable(value: &str) -> Option<String> {
    let named = value.strip_prefix('$')?;
    let inner = match named.strip_prefix('{') {
        Some(braced) => braced.strip_suffix('}')?,
        None => named,
    };
    let plain = !inner.is_empty()
        && inner
            .chars()
            .all(|letter| letter == '_' || letter.is_ascii_alphanumeric());
    if plain { Some(inner.to_owned()) } else { None }
}

/// The models the entry names, in the order it names them.
fn models(settings: &Map<String, Value>) -> Vec<ImportedModel> {
    let Some(Value::Array(listed)) = settings.get("models") else {
        return Vec::new();
    };
    let mut models = Vec::with_capacity(listed.len());
    for entry in listed {
        let Value::Object(model) = entry else {
            continue;
        };
        let Some(id) = text(model, "id") else {
            continue;
        };
        models.push(ImportedModel {
            id,
            context_tokens: number(model, "contextWindow"),
            max_output_tokens: number(model, "maxTokens").and_then(kernel::Ceiling::new),
        });
    }
    models
}

/// One string-valued key, or nothing when it is absent or is not a
/// string.
fn text(settings: &Map<String, Value>, key: &str) -> Option<String> {
    settings.get(key)?.as_str().map(str::to_owned)
}

/// One whole-number key, or nothing when it is absent or is not one.
fn number(settings: &Map<String, Value>, key: &str) -> Option<u64> {
    settings.get(key)?.as_u64()
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

    /// The shape pi's own documentation prints for a local server, with
    /// one model priced and one model bare.
    const LOCAL: &str = r#"{
      "providers": {
        "ollama": {
          "baseUrl": "http://localhost:11434/v1",
          "api": "openai-completions",
          "apiKey": "ollama",
          "models": [
            { "id": "llama3.1:8b", "contextWindow": 128000, "maxTokens": 32000 },
            { "id": "qwen2.5-coder:7b" }
          ]
        }
      }
    }"#;

    #[test]
    fn one_entry_becomes_one_endpoint_with_the_models_it_names() {
        let found = read(LOCAL).unwrap();
        assert!(found.unusable.is_empty());
        let row = &found.providers[0];
        assert_eq!(row.name.as_str(), "ollama");
        assert_eq!(row.base_url, "http://localhost:11434/v1");
        assert_eq!(row.wire, Spoken::Speaks(kernel::DialectKind::OpenAi));
        assert_eq!(row.models.len(), 2);
        assert_eq!(row.models[0].context_tokens, Some(128_000));
        assert_eq!(
            row.models[0].max_output_tokens,
            kernel::Ceiling::new(32_000)
        );
        assert_eq!(row.models[1].context_tokens, None);
        assert_eq!(row.models[1].max_output_tokens, None);
    }

    /// A placeholder a local server ignores is still a literal, and a
    /// literal stays where it was written.
    #[test]
    fn a_literal_key_stays_in_the_file_it_was_written_in() {
        assert_eq!(
            read(LOCAL).unwrap().providers[0].credential,
            Credential::NotCarried
        );
    }

    #[test]
    fn the_anthropic_wire_is_a_dialect_this_city_speaks() {
        let found = read(r#"{"providers":{"p":{"baseUrl":"u","api":"anthropic-messages"}}}"#);
        assert_eq!(
            found.unwrap().providers[0].wire,
            Spoken::Speaks(kernel::DialectKind::Anthropic)
        );
    }

    #[test]
    fn a_wire_this_city_has_no_dialect_for_is_carried_by_name() {
        let found = read(r#"{"providers":{"p":{"baseUrl":"u","api":"openai-responses"}}}"#);
        assert_eq!(
            found.unwrap().providers[0].wire,
            Spoken::Foreign {
                stated: "openai-responses".to_owned()
            }
        );
    }

    /// The four ways pi's own documentation says a key can be written,
    /// each read as what it is.
    #[test]
    fn a_credential_is_read_as_where_it_is_kept_and_never_as_its_value() {
        let cases = [
            (
                "$MY_API_KEY",
                Credential::Environment {
                    variable: "MY_API_KEY".to_owned(),
                },
            ),
            (
                "${MY_API_KEY}",
                Credential::Environment {
                    variable: "MY_API_KEY".to_owned(),
                },
            ),
            (
                "!op read 'op://v/i/c'",
                Credential::Command {
                    line: "op read 'op://v/i/c'".to_owned(),
                },
            ),
            ("$$literal-dollar", Credential::NotCarried),
            ("$!literal-bang", Credential::NotCarried),
            ("${A}_${B}", Credential::NotCarried),
            ("sk-plain", Credential::NotCarried),
        ];
        for (written, owed) in cases {
            let text = format!(r#"{{"providers":{{"p":{{"baseUrl":"u","apiKey":"{written}"}}}}}}"#);
            assert_eq!(
                read(&text).unwrap().providers[0].credential,
                owed,
                "{written}"
            );
        }
    }

    #[test]
    fn a_provider_with_no_base_url_is_reported_rather_than_dropped() {
        let found = read(r#"{"providers":{"p":{"api":"openai-completions"}}}"#).unwrap();
        assert!(found.providers.is_empty());
        assert_eq!(found.unusable[0].named, "p");
    }

    #[test]
    fn a_file_that_is_not_json_refuses_rather_than_importing_half_of_it() {
        assert!(read("{").is_err());
    }

    #[test]
    fn a_file_with_no_providers_object_imports_nothing() {
        assert_eq!(read("{}").unwrap(), Imported::default());
    }
}
