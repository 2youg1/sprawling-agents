// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { lookOf } from "./drop_refused";

describe("the files a drop did not keep", () => {
  // A send or an answer that broke off reaches the box as a refusal with
  // nothing said (`dropping.ts` `keep`); the line under the box must
  // still say why, or the file vanishes behind a dangling colon.
  test("each file has its own line, and an empty reason reads as the city not reached", () => {
    const look = lookOf(
      [
        { kind: "refused", name: "notes.md", said: "" },
        { kind: "refused", name: "notes.md", said: "too large" },
      ],
      "en",
    );
    expect(look).toEqual({
      lines: ["notes.md was not kept: not connected", "notes.md was not kept: too large"],
      wire: { role: "alert" },
    });
  });
});
