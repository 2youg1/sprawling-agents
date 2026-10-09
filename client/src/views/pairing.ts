// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the pairing page reads (`pairing.svelte`): the sentence that says
// why it is drawn, and the code as the door is sent it.

import type { Key } from "../core/lang";
import type { PairWhy } from "../core/local/entering";

// The sentence under the title beyond where the code is; a first
// pairing needs none.
export const WHY: Readonly<Record<PairWhy, Key | undefined>> = {
  first: undefined,
  open_used: "pair_open_used",
  session_lost: "pair_session_lost",
  keyless: "pair_keyless",
};

// The code as typed, the way a person copies it from a terminal: a
// space a copy picked up is not part of it, and the terminal prints it
// in lower case, so a person who typed capitals meant the same code.
// The dash between its two groups is part of it.
export function codeOf(typed: string): string {
  return typed.replace(/\s/g, "").toLowerCase();
}
