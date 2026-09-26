// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { closedUpTo } from "./prose";

// A streaming reply is split once per token: the closed head is laid out,
// the open tail is drawn raw. A head that ends inside a block would lay
// that block out twice as it grows, so these pin where the cut falls.
describe("closedUpTo", () => {
  const cut = (text: string) => {
    const at = closedUpTo(text);
    return [text.slice(0, at), text.slice(at)];
  };

  test("a list, a table and a code block closed while more is said", () => {
    const list = "- `crates/kernel`\n- **Ledger**\n\n";
    const table = "| 步骤 | 做什么 |\n|---|---|\n| 1 | 读 |\n\n";
    const code = "```rust\nfn main() {}\n\n```\n";
    const tail = "Then the next par";
    expect(cut(list + table + code + tail)).toEqual([list + table + code, tail]);
  });

  test("a code fence still open keeps its blank lines in the tail", () => {
    const head = "Before.\n\n";
    const open = "```rust\nfn a() {}\n\nfn b";
    expect(cut(head + open)).toEqual([head, open]);
  });

  test("a paragraph is closed by a blank line, not by a line break", () => {
    expect(cut("one line\nsame paragraph")).toEqual(["", "one line\nsame paragraph"]);
    expect(cut("# Title\nbody")).toEqual(["# Title\n", "body"]);
  });
});
