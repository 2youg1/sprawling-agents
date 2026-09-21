// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this person already configured in another harness, read once
//! (sprawling-SPEC.md section 8-71).
//!
//! A person who reaches this product has usually already told another
//! agent harness where their models are: a base URL, a wire, and the
//! environment variable their key sits in. Typing all of it a second
//! time is the first thing this product asks of them, and every
//! character of it is already on their disk.
//!
//! **One direction, once.** Files are read; nothing is written back,
//! and nothing is watched. Two tools that followed each other's
//! settings would be two authorities on one fact, and the person would
//! have no way to tell which of them had last won.
//!
//! **No credential is carried.** The rows say where the other harness
//! keeps a key - an environment variable, a command, or the file
//! itself - and never what the key is. Plaintext reaches the vault and
//! nowhere else, and an import that copied a key would put it in a
//! second place while telling nobody.
//!
//! The index holds no logic: `codex` and `pi` each own one grammar,
//! `machine` owns where the two files are, and `provider` owns what an
//! entry becomes.

mod codex;
mod machine;
mod pi;
mod provider;

pub(crate) use machine::scan;
pub(crate) use provider::{
    Credential, Harness, Imported, ImportedModel, ImportedProvider, Spoken, Unimportable, Unusable,
};
