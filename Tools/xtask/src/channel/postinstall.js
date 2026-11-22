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
const BUN_EXEC = process.execPath;

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
  const result = spawnSync(BUN_EXEC, ["pm", "bin", "--global"], {
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

function oursSymlink(dest, target) {
  try {
    const st = fs.lstatSync(dest);
    if (!st.isSymbolicLink()) {
      return false;
    }
    const current = fs.readlinkSync(dest);
    return path.resolve(path.dirname(dest), current) === path.resolve(target);
  } catch {
    return false;
  }
}

function oursWrapper(dest, target) {
  try {
    const text = fs.readFileSync(dest, "utf8");
    return text.includes(path.resolve(target)) && text.includes(path.resolve(BUN_EXEC));
  } catch {
    return false;
  }
}

function writeWrapper(dest, target) {
  const body =
    process.platform === "win32"
      ? `@echo off\r\n"${BUN_EXEC}" "${target}" %*\r\n`
      : `#!/bin/sh\nexec "${BUN_EXEC}" "${target}" "$@"\n`;
  fs.writeFileSync(dest, body, { mode: 0o755 });
  if (process.platform !== "win32") {
    fs.chmodSync(dest, 0o755);
  }
}

function installEntry(dir, name, target) {
  if (!dir || !fs.existsSync(target)) {
    return { ok: false, reason: "missing" };
  }
  try {
    fs.mkdirSync(dir, { recursive: true });
  } catch {
    return { ok: false, reason: "mkdir" };
  }
  const dest = path.join(dir, name);
  let exists = false;
  try {
    fs.lstatSync(dest);
    exists = true;
  } catch {
    exists = false;
  }
  if (exists) {
    if (oursSymlink(dest, target) || oursWrapper(dest, target)) {
      try {
        fs.unlinkSync(dest);
      } catch {
        return { ok: false, reason: "busy" };
      }
    } else {
      line(`kept existing ${dest} (not replaced; not this package's entry)`);
      return { ok: false, reason: "conflict" };
    }
  }
  try {
    writeWrapper(dest, target);
    return { ok: true, dest, kind: "wrapper" };
  } catch {
    return { ok: false, reason: "write" };
  }
}

function exportLine(dir) {
  if (process.platform === "win32") {
    return `$env:Path = "${dir};" + $env:Path`;
  }
  return `export PATH="${dir}:$PATH"`;
}

function persistHint(dir) {
  if (process.platform === "win32") {
    return `session only above; for new terminals, add ${dir} to your user Path`;
  }
  return `session only above; for new terminals, add that line to the file your login shell reads`;
}

const localBin = userLocalBin();
const mei = installEntry(localBin, "mei", MEI_SHIM);
const calyx = installEntry(localBin, "calyx", CALYX_SHIM);
const bunBin = bunGlobalBin();
const linked = mei.ok && calyx.ok;
const viaLocal = linked && pathHas(localBin);
const viaBunBin = pathHas(bunBin);

blank();
line("@sprawling/mei installed.");
line("Bun already linked the mei and calyx commands into its global bin");
line("(package bin field; no lifecycle script required).");
line("Open a NEW terminal, then run:");
line("  mei");
line("Headless one-shot:");
line('  mei -p "…"');
line("Face by name:");
line("  calyx");
if (linked) {
  line(`User links: ${localBin}`);
}
blank();

if (viaLocal || viaBunBin) {
  line("PATH already reaches the install.");
  line("A new terminal should resolve mei and calyx.");
  blank();
  process.exit(0);
}

line("Fix your PATH");
line("mei and calyx are installed, but this shell cannot see them yet.");
line("Run ONE export in this terminal (session only), then try mei:");
blank();
if (bunBin) {
  line(`  ${exportLine(bunBin)}`);
  line(`  (${persistHint(bunBin)})`);
}
if (localBin && linked) {
  line(`  ${exportLine(localBin)}`);
  line(`  (${persistHint(localBin)})`);
}
blank();
line("Ask Bun where globals land:");
line("  bun pm bin --global");
line("If a lifecycle script was skipped, that command still works after PATH is fixed.");
blank();
process.exit(0);
