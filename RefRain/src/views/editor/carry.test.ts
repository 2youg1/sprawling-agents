// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { carried } from "./carry";

// A reader who scrolled away from the caret switches reading: the line
// at the top of the editor, not the caret far below it, is what the
// preview opens at, so the page does not jump (A34).
describe("a change of reading", () => {
  const away = { cursor: 9000, top: 120 };

  test("from the source or the diff, the preview opens at the editor's top line", () => {
    expect([carried("source", "preview", away, 0), carried("diff", "preview", away, 0)]).toEqual([
      { kind: "preview_at", offset: 120 },
      { kind: "preview_at", offset: 120 },
    ]);
  });

  test("from the preview, the editor's top line is the preview's top block", () => {
    expect(carried("preview", "source", away, 640)).toEqual({ kind: "editor_at", byte: 640 });
  });

  test("between the source and the diff, or to the versions, nothing is carried", () => {
    expect([carried("source", "diff", away, 0), carried("preview", "versions", away, 0)]).toEqual([
      { kind: "nothing" },
      { kind: "nothing" },
    ]);
  });
});
