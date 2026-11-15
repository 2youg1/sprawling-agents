// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//
// The npm channel's entry point: find the binary for this machine and
// become it.
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

const { spawn } = require("node:child_process");

// The platforms the release builds, keyed the way node spells them.
// A machine outside this table is told what is built rather than handed
// a binary that cannot run, and the answer names the download page,
// which carries every archive whether or not npm has a package for it.
const PLATFORMS = {
  "win32 x64": { package: "@sprawling/sprawling-windows-x64", binary: "sprawling.exe" },
  "darwin arm64": { package: "@sprawling/sprawling-darwin-arm64", binary: "sprawling" },
  "linux x64": { package: "@sprawling/sprawling-linux-x64-musl", binary: "sprawling" },
};

const MEI_PLATFORMS = {
  "darwin arm64": { package: "@sprawling/mei-darwin-arm64", binary: "mei" },
  "linux x64": { package: "@sprawling/mei-linux-x64-musl", binary: "mei" },
};

const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";

// Both root packages carry this same selector. Their own manifest, not
// the current machine or a second generated script, chooses the product.
const packageManifest = (() => {
  try {
    return require("../package.json");
  } catch {
    return {};
  }
})();
const product =
  packageManifest.name === "@sprawling/mei"
    ? { command: "mei", root: packageManifest.name, platforms: MEI_PLATFORMS }
    : { command: "sprawling", root: "sprawling", platforms: PLATFORMS };

function die(message) {
  process.stderr.write(`${product.command}: ${message}\n`);
  process.exit(1);
}

// A package named at the version this run resolves it to, as npm spells
// that pair. npm pins every optional dependency to the root package's
// own version, so the root manifest is the one authority for which
// release is being looked for; in the published package it sits one
// directory above `bin/sprawling.js`. Where it cannot be read the name
// is given alone, so a failure still reports everything it knows.
function spec(name) {
  return typeof packageManifest.version === "string" ? `${name}@${packageManifest.version}` : name;
}

const key = `${process.platform} ${process.arch}`;
const row = product.platforms[key];
if (row === undefined) {
  die(
    `no binary is published for ${key}, so ${spec(product.root)} has ` +
      `nothing to run here. What is published: ` +
      `${Object.keys(product.platforms).join(", ")}. ` +
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
    `${spec(row.package)} is not installed, so there is no binary for ` +
      `${key}. Reinstall without omitting optional dependencies, ` +
      `or see ${RELEASES}.`,
  );
}

// `inherit`, so the city's console is this console: the process serves
// until Ctrl-C, and its output is not something to collect and reprint.
const child = spawn(binary, process.argv.slice(2), {
  stdio: "inherit",
  env: {
    ...process.env,
    SPRAWLING_INSTALL_CHANNEL: ["npm", "bun"].includes(process.env.SPRAWLING_INSTALL_CHANNEL)
      ? process.env.SPRAWLING_INSTALL_CHANNEL
      : __dirname.split(/[\\/]/).includes(".bun")
        ? "bun"
        : __dirname.split(/[\\/]/).includes("_npx")
          ? "npm"
          : "package",
  },
});
const signals = ["SIGINT", "SIGTERM", "SIGHUP"];
const forwards = new Map(signals.map((signal) => [signal, () => child.kill(signal)]));
for (const [signal, forward] of forwards) process.on(signal, forward);
child.on("error", (error) => die(`could not run ${binary}: ${error.message}`));
child.on("exit", (status, signal) => {
  for (const [name, forward] of forwards) process.removeListener(name, forward);
  if (signal !== null && process.platform !== "win32") {
    process.kill(process.pid, signal);
  } else {
    process.exit(status === null ? 1 : status);
  }
});
