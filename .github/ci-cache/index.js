// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The runner gives Node actions the cache service environment. Inherit it
// directly; neither an environment file nor a log should carry its token.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { isAbsolute, join } from "node:path";

function git(args, workspace) {
  const result = spawnSync("git", ["--no-replace-objects", ...args], {
    cwd: workspace,
    encoding: "utf8",
  });
  if (result.error || result.signal || result.status !== 0) {
    throw new Error(`git ${args[0]} could not verify the baseline (verification-base)`);
  }
  return result.stdout.trim();
}

function scopeBase(workspace) {
  const name = process.env.GITHUB_EVENT_NAME;
  if (!name) throw new Error("GITHUB_EVENT_NAME is missing (verification-event)");
  let base;
  if (name === "pull_request" || name === "push") {
    let event;
    try {
      event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, "utf8"));
    } catch {
      throw new Error("cannot read a valid GitHub event payload (verification-event)");
    }
    if (name === "pull_request") {
      base = event?.pull_request?.base?.sha;
    } else {
      base = event?.before;
      const releaseTag = typeof event?.ref === "string" && event.ref.startsWith("refs/tags/v");
      if (releaseTag && event.deleted === true) {
        throw new Error("a deleted release tag has no source tree to verify (verification-base)");
      }
      if (
        releaseTag &&
        event.created === true &&
        event.deleted === false &&
        base === "0".repeat(40)
      ) {
        base = git(["rev-parse", "--verify", "HEAD^"], workspace);
      }
    }
  } else {
    base = git(["merge-base", "origin/main", "HEAD"], workspace);
  }
  if (typeof base !== "string" || !/^[0-9a-f]{40}$/i.test(base) || /^0+$/.test(base)) {
    throw new Error("the event baseline is not a non-zero commit ID (verification-base)");
  }
  git(["cat-file", "-e", `${base}^{commit}`], workspace);
  return base;
}

const workspace = process.env.GITHUB_WORKSPACE;
try {
  if (!workspace || !isAbsolute(workspace)) {
    throw new Error(
      "GITHUB_WORKSPACE must name an absolute checkout path (verification-workspace)",
    );
  }
  const base = scopeBase(workspace);
  const result = spawnSync("just", ["verify-sources", "--full", "--scope-base", base], {
    cwd: workspace,
    env: {
      ...process.env,
      SPRAWLING_CI_CACHE_ADAPTER: join(import.meta.dirname, "cache.ts"),
    },
    stdio: "inherit",
  });
  if (result.error) {
    throw new Error("could not launch just verify-sources (verification-launch)");
  }
  if (result.signal) {
    throw new Error(`source verification terminated by ${result.signal} (verification-signal)`);
  }
  process.exitCode = result.status ?? 2;
} catch (error) {
  console.error(
    "```toml\n" +
      'code = "E_CI_SOURCE"\n' +
      'action = "run the complete source verification graph"\n' +
      'subject = "checkout, event baseline and verifier process"\n' +
      `path = ${JSON.stringify(workspace ?? "GITHUB_WORKSPACE")}\n` +
      `value = ${JSON.stringify(error.message)}\n` +
      'rule = "use a non-zero available commit baseline and preserve verification failures"\n' +
      'recovery = "Run just prereqs; restore checkout with fetch-depth: 0; rerun ci.yml on this ref."\n' +
      "```",
  );
  process.exitCode = 2;
}
