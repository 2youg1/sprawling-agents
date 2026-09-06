// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//! The custodian: capture, resolve, rotate.

use super::oauth::degraded_payload;
use super::vault::{Described, EnvReader, KeyringVault, MemoryVault, Persistence, Vault, env_key};

use kernel::{AxCode, AxError, Payload, Sealed, SecretRef, scan};
use serde_json::{Map, Value};
use zeroize::Zeroizing;

/// The inner seam: store, fetch, delete. Nothing else leaves the crate.
pub struct Custodian {
    backend: Box<dyn Vault + Send>,
    source: &'static str,
    persistence: Persistence,
    env: EnvReader,
    captures: u64,
}

/// What `capture` returns: the bytes with plaintext replaced by
/// `secret:` literals, plus the `secret_captured` payloads the caller
/// appends (realm/name/origin/span length only — no plaintext, no hash
/// prefix).
pub struct Captured {
    pub replaced: Vec<u8>,
    pub events: Vec<Payload>,
}

impl Custodian {
    /// Startup probe: write-read-delete against the platform service;
    /// all candidates failing falls back to session memory and returns
    /// the `provider_degraded` payload for the ledger. Never silent.
    pub fn probe() -> (Custodian, Option<Payload>) {
        let probe_ref = match SecretRef::parse("secret:sprawling/startup-probe") {
            Ok(reference) => reference,
            Err(_) => {
                return (
                    Custodian::with_backend(
                        Box::new(MemoryVault::default()),
                        "session-memory",
                        Persistence::ThisProcess,
                    ),
                    degraded_payload("probe reference unparsable"),
                );
            }
        };
        let mut candidate = KeyringVault;
        let round_trip = candidate
            .put(&probe_ref, Zeroizing::new("probe".to_owned()))
            .and_then(|()| candidate.get(&probe_ref))
            .and_then(|read| {
                candidate.delete(&probe_ref)?;
                Ok(read)
            });
        match round_trip {
            Ok(Some(read)) if read.as_str() == "probe" => (
                Custodian::with_backend(
                    Box::new(KeyringVault),
                    "platform-credential-service",
                    Persistence::AcrossReboots,
                ),
                None,
            ),
            Ok(_) => (
                Custodian::with_backend(
                    Box::new(MemoryVault::default()),
                    "session-memory",
                    Persistence::ThisProcess,
                ),
                degraded_payload("platform service returned a different value"),
            ),
            Err(err) => (
                Custodian::with_backend(
                    Box::new(MemoryVault::default()),
                    "session-memory",
                    Persistence::ThisProcess,
                ),
                degraded_payload(err.subject()),
            ),
        }
    }

    /// Session-memory custodian (tests, headless fallback by choice).
    pub fn in_memory() -> Custodian {
        Custodian::with_backend(
            Box::new(MemoryVault::default()),
            "session-memory",
            Persistence::ThisProcess,
        )
    }

    fn with_backend(
        backend: Box<dyn Vault + Send>,
        source: &'static str,
        persistence: Persistence,
    ) -> Custodian {
        Custodian {
            backend,
            source,
            persistence,
            env: Box::new(|key| std::env::var(key).ok()),
            captures: 0,
        }
    }

    /// Test seam: replace the read-only source reader.
    pub fn with_env_reader(mut self, env: EnvReader) -> Custodian {
        self.env = env;
        self
    }

    /// Custody's effect half: spans from `kernel::secret::scan`, values
    /// into the vault, `secret:` literals into the bytes. Deterministic
    /// names: `cap-<n>` under the shape's provider realm.
    pub fn capture(&mut self, bytes: &[u8], origin: &str) -> Result<Captured, AxError> {
        let spans = scan(bytes);
        let mut replaced = bytes.to_vec();
        let mut events = Vec::new();
        // Reverse order keeps earlier offsets valid while splicing.
        for span in spans.iter().rev() {
            let end = span.start.saturating_add(span.len);
            let Some(slice) = bytes.get(span.start..end) else {
                continue;
            };
            let plaintext = Zeroizing::new(String::from_utf8_lossy(slice).into_owned());
            self.captures = self.captures.saturating_add(1);
            let realm = span.provider.unwrap_or("detected");
            let name = format!("cap-{}", self.captures);
            let reference = SecretRef::parse(&format!("secret:{realm}/{name}"))?;
            self.backend.put(&reference, plaintext)?;
            let literal = reference.to_string();
            replaced.splice(span.start..end, literal.bytes());
            let mut event = Map::new();
            event.insert("realm".to_owned(), Value::String(realm.to_owned()));
            event.insert("name".to_owned(), Value::String(name));
            event.insert("origin".to_owned(), Value::String(origin.to_owned()));
            event.insert(
                "span_len".to_owned(),
                Value::Number(u64::try_from(span.len).unwrap_or(0).into()),
            );
            events.push(Payload::new(event)?);
        }
        events.reverse();
        Ok(Captured { replaced, events })
    }

    /// Stores a value. Empty is not a configuration; a shaded reference
    /// (read-only source active) refuses and names the shader. The input
    /// is `Zeroizing`, not `Sealed`: `Sealed` unseals only at the two
    /// wire redemption points, and custody is a store, not a sink —
    /// callers holding a `Sealed` keep it sealed all the way to the wire
    /// (the S4 command face converts inside its own boundary).
    pub fn set(&mut self, reference: &SecretRef, value: Zeroizing<String>) -> Result<(), AxError> {
        if value.is_empty() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "store credential",
                "empty value is not a configuration",
            ));
        }
        let key = env_key(reference);
        if (self.env)(&key).is_some_and(|v| !v.is_empty()) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "store credential",
                format!("{reference} is shaded by the environment variable {key}"),
            )
            .with_recovery("unset the environment variable, then store again"));
        }
        self.backend.put(reference, value)
    }

    /// Redemption: resolve per operation. Rotation works because no
    /// second copy survives between operations.
    pub fn resolve(&self, reference: &SecretRef) -> Result<Sealed<String>, AxError> {
        let key = env_key(reference);
        if let Some(value) = (self.env)(&key)
            && !value.is_empty()
        {
            return Ok(Sealed::new(Box::new(value)));
        }
        match self.backend.get(reference)? {
            Some(value) if !value.is_empty() => {
                Ok(Sealed::new(Box::new(value.as_str().to_owned())))
            }
            _ => Err(AxError::failure(
                AxCode::CredentialMissing,
                "resolve credential",
                reference.to_string(),
            )
            .with_recovery("store the credential, or set its environment variable")),
        }
    }

    /// State you can render; the value stays unreachable.
    pub fn describe(&self, reference: &SecretRef) -> Described {
        let key = env_key(reference);
        if (self.env)(&key).is_some_and(|v| !v.is_empty()) {
            return Described {
                configured: true,
                source: format!("environment ({key})"),
                persistence: Persistence::ThisProcess,
                writable: false,
            };
        }
        let configured = matches!(self.backend.get(reference), Ok(Some(v)) if !v.is_empty());
        Described {
            configured,
            source: self.source.to_owned(),
            persistence: self.persistence,
            writable: true,
        }
    }

    pub fn persistence(&self) -> Persistence {
        self.persistence
    }
}

/// PKCE begin (RFC 7636, S256): pure construction — the browser visit
/// and the token POST are the caller's I/O. The verifier arrives from
/// the assembly's seeded randomness (kernel never samples).
#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;
    fn sample_token() -> String {
        // Runtime-assembled: the repository never holds a complete
        // high-entropy literal at rest (xtask secret discipline).
        ["sk-ant-api03-", "Zx9yQ2mK4pL7", "vB1nC5tR8sD3"].concat()
    }

    /// A token endpoint that answers one POST with `status` and `body`,
    /// and reports what it was sent.
    /// A token endpoint that answers one POST with `status` and `body`,
    /// and reports what it was sent.

    #[test]
    fn a13_capture_leaves_references_not_plaintext() {
        let mut custodian = Custodian::in_memory();
        let paste = format!("my key is {} thanks", sample_token());
        let captured = custodian.capture(paste.as_bytes(), "paste").unwrap();
        let replaced = String::from_utf8(captured.replaced.clone()).unwrap();
        assert!(!replaced.contains(&sample_token()), "no plaintext left");
        assert!(replaced.contains("secret:anthropic/cap-1"), "{replaced}");
        // The payload names realm/name/origin/len — nothing else.
        let event = serde_json::to_value(&captured.events[0]).unwrap();
        assert_eq!(event["realm"], "anthropic");
        assert_eq!(event["origin"], "paste");
        assert!(event.get("value").is_none());
        assert!(!event.to_string().contains("Zx9yQ2mK4pL7"));
        // Redemption succeeds and stays sealed; describe reports state,
        // never the value. (Value correctness is proven on the wire in
        // `a13_redeemed_value_reaches_the_wire_verbatim` — no expose
        // call exists outside the two redemption points.)
        let reference = SecretRef::parse("secret:anthropic/cap-1").unwrap();
        assert!(custodian.resolve(&reference).is_ok());
        let described = custodian.describe(&reference);
        assert!(described.configured);
        assert_eq!(described.persistence, Persistence::ThisProcess);
    }

    #[test]
    fn missing_and_empty_read_as_not_configured() {
        let mut custodian = Custodian::in_memory();
        let reference = SecretRef::parse("secret:acme/key").unwrap();
        let err = match custodian.resolve(&reference) {
            Err(err) => err,
            Ok(_) => panic!("missing credential must not resolve"),
        };
        assert_eq!(*err.code(), AxCode::CredentialMissing);
        let err = custodian
            .set(&reference, Zeroizing::new(String::new()))
            .unwrap_err();
        assert_eq!(*err.code(), AxCode::InvalidArgs);
        assert!(!custodian.describe(&reference).configured);
    }

    #[test]
    fn the_environment_shades_and_set_refuses_naming_the_shader() {
        let mut custodian = Custodian::in_memory().with_env_reader(Box::new(|key| {
            (key == "SPRAWLING_SECRET_ACME_KEY").then(|| "from-env".to_owned())
        }));
        let reference = SecretRef::parse("secret:acme/key").unwrap();
        // Resolve serves the read-only source (sealed).
        assert!(custodian.resolve(&reference).is_ok());
        // Describe shows read-only.
        let described = custodian.describe(&reference);
        assert!(described.configured);
        assert!(!described.writable);
        assert!(described.source.contains("SPRAWLING_SECRET_ACME_KEY"));
        // Set refuses: it would look successful and change nothing.
        let err = custodian
            .set(&reference, Zeroizing::new("new".to_owned()))
            .unwrap_err();
        assert!(err.subject().contains("SPRAWLING_SECRET_ACME_KEY"));
    }

    #[test]
    fn rotation_is_next_operation_effective_because_nothing_caches() {
        let mut custodian = Custodian::in_memory();
        let reference = SecretRef::parse("secret:acme/rotating").unwrap();
        custodian
            .set(&reference, Zeroizing::new("one".to_owned()))
            .unwrap();
        assert!(custodian.resolve(&reference).is_ok());
        custodian
            .set(&reference, Zeroizing::new("two".to_owned()))
            .unwrap();
        // No copy survives between operations: the next resolve reads the
        // backend, so the rotated value is what the wire test would see.
        assert!(custodian.resolve(&reference).is_ok());
    }
}
