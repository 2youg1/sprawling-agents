// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { lockOf, lockShare, withLock } from "./context_lock";

describe("context_lock", () => {
  test("reads and rewrites the RULES key", () => {
    const base = 'confidential = false\nwrite = "everything"\n';
    expect(lockOf(base)).toBeNull();
    const set = withLock(base, 262_144);
    expect(lockOf(set)).toBe(262_144);
    expect(set).toContain("context_lock_tokens = 262144");
    expect(lockOf(withLock(set, null))).toBeNull();
  });

  test("inserts the lock in the root table before a nested table", () => {
    const withChoice = [
      'confidential = false',
      'write = "everything"',
      "",
      "[workspace_models.choice]",
      'endpoint = "house"',
      'model = "ordinary"',
      'effort = "medium"',
      "",
    ].join("\n");
    const set = withLock(withChoice, 262_144);
    expect(lockOf(set)).toBe(262_144);
    const choiceAt = set.indexOf("[workspace_models.choice]");
    const lockAt = set.indexOf("context_lock_tokens");
    expect(lockAt).toBeGreaterThanOrEqual(0);
    expect(lockAt).toBeLessThan(choiceAt);
    expect(set.slice(choiceAt)).not.toContain("context_lock_tokens");
  });

  test("reads TOML integers that use underscores", () => {
    const text = 'confidential = false\ncontext_lock_tokens = 262_144\n';
    expect(lockOf(text)).toBe(262_144);
    expect(withLock(text, 1000)).toContain("context_lock_tokens = 1000");
    expect(withLock(text, 1000).match(/context_lock_tokens/g)).toHaveLength(1);
  });

  test("reports the lock as a share of the model window", () => {
    expect(lockShare(262_144, 1_050_000)).toBe(24);
    expect(lockShare(0, 1_000)).toBeNull();
  });
});
