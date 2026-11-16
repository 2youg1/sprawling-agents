// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { reachOf } from "./staleness";

describe("a file taken back from a checkpoint", () => {
  test("makes the working tree, the listing and the document stale", () => {
    expect([
      reachOf("git_status", "file_restored"),
      reachOf("listing", "file_restored"),
      reachOf("document", "file_restored"),
    ]).toEqual(["every", "every", "every"]);
  });

  test("leaves the commit pages alone, because it commits nothing", () => {
    expect(reachOf("commits", "file_restored")).toBe("none");
  });
});
