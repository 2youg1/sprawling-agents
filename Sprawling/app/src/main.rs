// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod install;
mod wire_client;

use std::process::ExitCode;

#[cfg(target_env = "musl")]
#[global_allocator]
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

include!(concat!(env!("OUT_DIR"), "/client_embed.rs"));

const DEPENDENCIES: &str = include_str!(concat!(env!("OUT_DIR"), "/deps.txt"));

#[path = "main/calling.rs"]
mod calling;
#[path = "main/check.rs"]
mod check;
#[path = "main/city.rs"]
mod city;
#[path = "main/data.rs"]
mod data;
#[path = "main/desktop.rs"]
mod desktop;
#[path = "main/dispatch.rs"]
mod dispatch;
#[path = "main/exit.rs"]
mod exit;
#[path = "main/gauge.rs"]
mod gauge;
#[path = "main/grammar.rs"]
mod grammar;
#[path = "main/playback.rs"]
mod playback;
#[path = "main/refusal.rs"]
mod refusal;
#[path = "main/router.rs"]
mod router;
#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
#[path = "main/verbs.rs"]
mod verbs;
#[path = "main/version.rs"]
mod version;
#[path = "main/view.rs"]
mod view;
#[path = "main/whose.rs"]
mod whose;

fn main() -> ExitCode {
    router::main()
}
