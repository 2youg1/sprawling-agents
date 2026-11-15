// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
//
// The npm channel's entry point: find the binary for this machine and
// become it. Never installs. For @sprawling/mei: a TTY without `-p`
// becomes Calyx; `-p` or a non-TTY stdin becomes the headless core.

"use strict";

const { spawn } = require("node:child_process");
const path = require("node:path");

const PLATFORMS = {
  "win32 x64": { package: "@sprawling/sprawling-windows-x64", binary: "sprawling.exe" },
  "darwin arm64": { package: "@sprawling/sprawling-darwin-arm64", binary: "sprawling" },
  "linux x64": { package: "@sprawling/sprawling-linux-x64-musl", binary: "sprawling" },
};

const MEI_PLATFORMS = {
  "win32 x64": {
    package: "@sprawling/mei-windows-x64",
    core: "mei.exe",
    face: "calyx.exe",
  },
  "darwin arm64": {
    package: "@sprawling/mei-darwin-arm64",
    core: "mei",
    face: "calyx",
  },
  "linux x64": {
    package: "@sprawling/mei-linux-x64-musl",
    core: "mei",
    face: "calyx",
  },
};

const RELEASES = "https://github.com/2youg1/sprawling-agents/releases";

const packageManifest = (() => {
  try {
    return require("../package.json");
  } catch {
    return {};
  }
})();
const isMei = packageManifest.name === "@sprawling/mei";
const product = isMei
  ? { command: "mei", root: packageManifest.name, platforms: MEI_PLATFORMS }
  : { command: "sprawling", root: "sprawling", platforms: PLATFORMS };

function die(message) {
  process.stderr.write(`${product.command}: ${message}\n`);
  process.exit(1);
}

function spec(name) {
  return typeof packageManifest.version === "string" ? `${name}@${packageManifest.version}` : name;
}

function wantsHeadless(argv) {
  if (argv.includes("-p")) return true;
  if (!process.stdin.isTTY) return true;
  return false;
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

function resolveBinary(packageName, binaryName) {
  try {
    return require.resolve(`${packageName}/bin/${binaryName}`);
  } catch {
    die(
      `${spec(packageName)} is not installed, so there is no binary for ` +
        `${key}. Reinstall without omitting optional dependencies, ` +
        `or see ${RELEASES}.`,
    );
  }
}

let binary;
let childEnv = { ...process.env };
if (isMei) {
  const core = resolveBinary(row.package, row.core);
  if (wantsHeadless(process.argv.slice(2))) {
    binary = core;
  } else {
    binary = resolveBinary(row.package, row.face);
    childEnv.MEI_BIN = core;
  }
} else {
  binary = resolveBinary(row.package, row.binary);
}

childEnv.SPRAWLING_INSTALL_CHANNEL = ["npm", "bun"].includes(process.env.SPRAWLING_INSTALL_CHANNEL)
  ? process.env.SPRAWLING_INSTALL_CHANNEL
  : __dirname.split(/[\\/]/).includes(".bun")
    ? "bun"
    : __dirname.split(/[\\/]/).includes("_npx")
      ? "npm"
      : "package";

const child = spawn(binary, process.argv.slice(2), {
  stdio: "inherit",
  env: childEnv,
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
