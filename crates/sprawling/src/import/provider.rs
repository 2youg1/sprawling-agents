// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One provider entry of another harness, said in this city's own terms
//! (sprawling-SPEC.md section 8-71).
//!
//! The two harnesses state the same four facts about an endpoint under
//! different names: what it is called, where it is, which request shape
//! it answers in, and where its credential is kept. This module is the
//! one place those four become values this city can act on, so the two
//! readers below differ only in the grammar they walk.

use channels::ProviderName;
use kernel::{Ceiling, DialectKind};

/// Which harness on this machine an entry was read from.
///
/// Carried on every row because the person is told where a suggestion
/// came from: they configured one of these themselves, and an endpoint
/// offered with no provenance is one they have to go and look for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Harness {
    /// OpenAI Codex, which keeps `[model_providers.*]` in a TOML file.
    Codex,
    /// pi, which keeps `providers` in a JSON file.
    Pi,
}

impl Harness {
    /// What this harness is called where a person reads it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Harness::Codex => "codex",
            Harness::Pi => "pi",
        }
    }
}

/// Which request shape the other harness says this endpoint answers in.
///
/// Three cases rather than an `Option<DialectKind>`, because "this
/// harness names a wire we do not speak" and "this harness names no
/// wire" lead a person to different actions: the first says which one
/// their other tool was using, and the second says nothing was written
/// down at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Spoken {
    /// A wire this city has a dialect for.
    Speaks(DialectKind),
    /// A wire this city has no dialect for, carried by its own spelling
    /// so the person reads which one they were using.
    Foreign { stated: String },
    /// The entry names no wire, so the person chooses.
    Unstated,
}

/// Where the other harness keeps this endpoint's credential.
///
/// **No credential value is ever carried in any of these.** A key that
/// reached this city outside the vault would be a key in a second
/// place, and the import exists to save a person typing a URL rather
/// than to move their secrets for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Credential {
    /// The other harness reads the key from this environment variable.
    Environment { variable: String },
    /// The other harness runs this command line and takes what it
    /// prints. The line is carried; running it is nobody's here.
    Command { line: String },
    /// The entry states the credential in a form this city does not
    /// read - a literal, or a value assembled from several variables.
    /// The value stays in the other harness's file.
    NotCarried,
    /// The entry states no credential, which is what a local server
    /// that asks for none looks like.
    Unstated,
}

/// One model an entry admits, with the two figures no model list
/// returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportedModel {
    pub(crate) id: String,
    /// The window, when the entry states one. Absent rather than zero:
    /// zero is how the wire spells "nobody stated a window", and a
    /// reader of this value has not reached the wire yet.
    pub(crate) context_tokens: Option<u64>,
    pub(crate) max_output_tokens: Option<Ceiling>,
}

/// One endpoint another harness on this machine is already configured
/// with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportedProvider {
    pub(crate) harness: Harness,
    pub(crate) name: ProviderName,
    pub(crate) base_url: String,
    pub(crate) wire: Spoken,
    pub(crate) credential: Credential,
    /// The models the entry names, which is what this city would admit.
    /// Empty is an entry that names none, and admits whatever the
    /// endpoint serves.
    pub(crate) models: Vec<ImportedModel>,
}

/// Why one entry produced no endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unimportable {
    /// The entry states no base URL, so there is no endpoint to attach.
    NoAddress,
    /// The entry is not a table of settings at all.
    Malformed,
    /// The name the other harness gave it is not a name this city can
    /// carry, with what the refusal said to do about it.
    NameRefused { recovery: String },
}

/// One entry that produced no endpoint, named so that a reader sees it
/// was there.
///
/// Reported rather than dropped: a person who configured six providers
/// in another tool and is offered four has to be told which two were
/// left out, or the import reads as a defect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unusable {
    pub(crate) harness: Harness,
    pub(crate) named: String,
    pub(crate) because: Unimportable,
}

/// What one entry states, before this city has judged it.
///
/// The five values travel together because they are one entry; passing
/// them one at a time would put the judgment below behind a signature
/// nobody can read.
pub(crate) struct Stated {
    pub(crate) named: String,
    pub(crate) base_url: Option<String>,
    pub(crate) wire: Spoken,
    pub(crate) credential: Credential,
    pub(crate) models: Vec<ImportedModel>,
}

/// What one pass over this machine found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Imported {
    pub(crate) providers: Vec<ImportedProvider>,
    pub(crate) unusable: Vec<Unusable>,
}

impl Imported {
    /// Takes everything another pass found.
    pub(crate) fn absorb(&mut self, other: Imported) {
        self.providers.extend(other.providers);
        self.unusable.extend(other.unusable);
    }

    /// Files one entry under whichever of the two lists it belongs in.
    ///
    /// The one place an entry is judged, shared by both readers: a
    /// second copy of these two refusals would let one harness import
    /// what the other rejected.
    pub(crate) fn take(&mut self, harness: Harness, stated: Stated) {
        let named = stated.named;
        let Some(base_url) = stated.base_url else {
            self.unusable.push(Unusable {
                harness,
                named,
                because: Unimportable::NoAddress,
            });
            return;
        };
        match ProviderName::parse(&named) {
            Ok(name) => self.providers.push(ImportedProvider {
                harness,
                name,
                base_url,
                wire: stated.wire,
                credential: stated.credential,
                models: stated.models,
            }),
            Err(refusal) => self.unusable.push(Unusable {
                harness,
                named,
                because: Unimportable::NameRefused {
                    recovery: refusal.recovery().to_owned(),
                },
            }),
        }
    }

    /// Files one entry that is not a table of settings at all.
    pub(crate) fn malformed(&mut self, harness: Harness, named: String) {
        self.unusable.push(Unusable {
            harness,
            named,
            because: Unimportable::Malformed,
        });
    }
}
