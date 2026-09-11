#!/usr/bin/env node
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//
// The npm channel's entry point: find the binary for this machine and
// become it.
//
// **The first line is load-bearing.** npm writes a shell wrapper for
// every `bin`, and that wrapper reads the shebang to decide what runs
// the file; without one it executes the JavaScript as a shell script,
// which on a Unix shell fails on line 1 and on Windows fails silently
// with exit 0 - a `bunx sprawling` that prints nothing and reports
// success.
//
// **This script never installs anything.** Where a downloaded archive
// puts the binary, and what happens to PATH, belongs to `sprawling
// install`; where an npm package puts it belongs to npm or bun. Two
// installers choosing a directory would be two authorities for one
// rule, so this one resolves, execs, and passes the exit code back.
//
// One package per platform, listed as optional dependencies of the root
// package: npm and bun install only the row whose `os` and `cpu` match,
// so a Windows machine never downloads a macOS binary.

"use strict";

const { spawnSync } = require("node:child_process");

// The platforms the release builds, keyed the way node spells them.
// A machine outside this table is told what is built rather than handed
// a binary that cannot run, and the answer names the download page,
// which carries every archive whether or not npm has a package for it.
const PLATFORMS = {
  "win32 x64": { package: "@sprawling/windows-x64", binary: "sprawling.exe" },
  "darwin arm64": { package: "@sprawling/darwin-arm64", binary: "sprawling" },
  "linux x64": { package: "@sprawling/linux-x64-musl", binary: "sprawling" },
};

const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";

function die(message) {
  process.stderr.write(`sprawling: ${message}\n`);
  process.exit(1);
}

const key = `${process.platform} ${process.arch}`;
const row = PLATFORMS[key];
if (row === undefined) {
  die(
    `no binary is published for ${key}. What is published: ` +
      `${Object.keys(PLATFORMS).join(", ")}. ` +
      `Build from source, or see ${RELEASES}.`,
  );
}

let binary;
try {
  binary = require.resolve(`${row.package}/bin/${row.binary}`);
} catch {
  // The optional dependency did not install. The usual causes are an
  // `--omit=optional` install and a lockfile copied from another
  // platform, and both are fixed the same way, so the message says the
  // fix rather than guessing which one happened.
  die(
    `${row.package} is not installed, so there is no binary for ${key}. ` +
      `Reinstall without omitting optional dependencies, or see ${RELEASES}.`,
  );
}

// `inherit`, so the city's console is this console: the process serves
// until Ctrl-C, and its output is not something to collect and reprint.
const finished = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (finished.error !== undefined && finished.error !== null) {
  die(`could not run ${binary}: ${finished.error.message}`);
}
// A process killed by a signal has no status. Reporting that as success
// would tell a script that a city shut down cleanly when it was killed.
process.exit(finished.status === null ? 1 : finished.status);
