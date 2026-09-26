// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { EMPTY, encode } from "./draft";

// A draft with every field but its label filled in, so the label is the
// only thing the verdict can turn on.
const labelled = (label: string) => encode({ ...EMPTY, label, command: "npx mcp-docs" }, []);

describe("a server label", () => {
  test("is judged by the city's grammar, not a page's", () => {
    expect([labelled("9lives").kind, labelled("docs2").kind, labelled("my-docs").kind, labelled("my_docs").kind]).toEqual([
      "ready",
      "ready",
      "blocked",
      "blocked",
    ]);
  });
});
