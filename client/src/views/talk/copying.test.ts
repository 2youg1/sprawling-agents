// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { copiedSource } from "./copying";

const SOURCE = "# 标题\n\n- **a**\n- b\n\n```rust\nfn main() {}\n```";
const DRAWN = "标题\na\nb\nfn main() {}";

describe("copying a reply", () => {
  test("the whole reply copies as the Markdown the model wrote", () => {
    expect(copiedSource("标题\n\na\n\nb\nfn main() {}\n", DRAWN, SOURCE)).toBe(SOURCE);
  });

  test("words chosen inside the reply are the browser's to copy", () => {
    expect([copiedSource("a\nb", DRAWN, SOURCE), copiedSource("", "", SOURCE)]).toEqual([null, null]);
  });
});
