// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const ROOT = path.resolve(__dirname, "..");
const MEI_SHIM = path.join(ROOT, "bin", "mei.js");
const CALYX_SHIM = path.join(ROOT, "bin", "calyx.js");

function line(text) {
  process.stdout.write(`${text}\n`);
}

function blank() {
  process.stdout.write("\n");
}

function userLocalBin() {
  if (process.platform === "win32") {
    return null;
  }
  return path.join(os.homedir(), ".local", "bin");
}

function bunGlobalBin() {
  const result = spawnSync("bun", ["pm", "bin", "--global"], {
    encoding: "utf8",
    env: process.env,
  });
  if (result.status !== 0) {
    return "";
  }
  return (result.stdout || "").trim();
}

function pathHas(dir) {
  if (!dir) {
    return false;
  }
  const wanted = path.resolve(dir);
  return (process.env.PATH || "")
    .split(path.delimiter)
    .filter(Boolean)
    .some((entry) => {
      try {
        return path.resolve(entry) === wanted;
      } catch {
        return false;
      }
    });
}

function linkInto(dir, name, target) {
  if (!dir || !fs.existsSync(target)) {
    return null;
  }
  try {
    fs.mkdirSync(dir, { recursive: true });
  } catch {
    return null;
  }
  const dest = path.join(dir, name);
  try {
    fs.lstatSync(dest);
    fs.unlinkSync(dest);
  } catch {}
  try {
    fs.symlinkSync(target, dest);
    return dest;
  } catch {
    try {
      fs.copyFileSync(target, dest);
      fs.chmodSync(dest, 0o755);
      return dest;
    } catch {
      return null;
    }
  }
}

function exportLine(dir) {
  if (process.platform === "win32") {
    return `$env:Path = "${dir};" + $env:Path`;
  }
  return `export PATH="${dir}:$PATH"`;
}

const localBin = userLocalBin();
const linkedMei = linkInto(localBin, "mei", MEI_SHIM);
const linkedCalyx = linkInto(localBin, "calyx", CALYX_SHIM);
const bunBin = bunGlobalBin();
const reachable = pathHas(localBin) || pathHas(bunBin);

blank();
line("@sprawling/mei installed.");
line("Open a NEW terminal, then run:");
line("  mei");
line("Headless one-shot:");
line('  mei -p "…"');
line("Face by name:");
line("  calyx");
if (linkedMei || linkedCalyx) {
  line(`User links: ${localBin}`);
}
blank();

if (reachable) {
  line("PATH already reaches the install.");
  line("A new terminal should resolve mei and calyx.");
  blank();
  process.exit(0);
}

line("Fix your PATH");
line("mei and calyx are installed, but this shell cannot see them yet.");
line("Add ONE directory below to PATH, then open a NEW terminal:");
blank();
if (bunBin) {
  line(`  ${exportLine(bunBin)}`);
}
if (localBin) {
  line(`  ${exportLine(localBin)}`);
}
blank();
line("Ask Bun where globals land:");
line("  bun pm bin --global");
blank();
process.exit(0);
