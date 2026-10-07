// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the folded reasoning of a round is given. The fold is the
// engine's own `<details>`, so the look carries the whole disclosure and
// the seat - the thread for the round being said, a round for one that
// is over - only says whether it starts open.

export interface ReasoningLook {
  // Both already in the person's language: the fold's name, and how
  // long the reasoning is.
  readonly label: string;
  readonly length: string;
  readonly text: string;
  // The round being said keeps its reasoning open while it grows; a
  // finished round folds it, because the reply is what is read.
  readonly open: boolean;
}
