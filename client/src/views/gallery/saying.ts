// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A live turn at several lengths, drawn by the thread's own `Saying`:
// the faint edge at the growing end, and - in the last - a list, a
// table and a code block already closed and laid out while the
// paragraph after them is still arriving.
export const SAYING = [
  "on",
  "on it — reading the city",
  "on it — reading the city, then writing the plan it asks for",
  [
    "- `crates/kernel`",
    "- **Ledger**",
    "",
    "| step | what |",
    "|---|---|",
    "| 1 | read |",
    "",
    "```rust",
    "fn main() {}",
    "```",
    "then the plan it asks",
  ].join("\n"),
];
