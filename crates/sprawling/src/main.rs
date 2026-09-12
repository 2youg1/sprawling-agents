// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! CLI entry. Subcommands land with their stages and are refused honestly
//! until then — a refusal that names what is missing beats a stub that
//! pretends (sprawling-SPEC.md). Live now: status, replay, init, serve,
//! export, restore, resume, fork.

// The city harness is the library half of this package (`src/lib.rs`);
// these two are the binary's own. `install` puts this executable where a
// shell will find it, and `wire_client` talks to a served city from a
// terminal - both are about the command line rather than about a city.
mod install;
mod wire_client;

use std::process::ExitCode;

// CLIENT_FILES and CLIENT_COMPLETE: the gzipped client bundle the build
// wrote, and whether it is the whole client or only the page shell.
include!(concat!(env!("OUT_DIR"), "/client_embed.rs"));

/// Every crate this binary is built from, `name version` per line -
/// the embedded half of the bill of materials (`xtask sbom` writes the
/// CycloneDX file half).
const DEPENDENCIES: &str = include_str!(concat!(env!("OUT_DIR"), "/deps.txt"));

#[path = "main/city.rs"]
mod city;
#[path = "main/data.rs"]
mod data;
#[path = "main/router.rs"]
mod router;
#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
#[path = "main/version.rs"]
mod version;
#[path = "main/whose.rs"]
mod whose;

fn main() -> ExitCode {
    router::main()
}
