// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the words of a turn still arriving give their look
// (`saying.look.svelte`): who is speaking, the blocks the city has laid
// out, and the open tail split where it starts to fade.

import type { Snippet } from "svelte";

export interface SayingLook {
  readonly who: string;
  // The blocks that can no longer change, laid out by `refrain/laid`.
  readonly laid: Snippet;
  // The open tail: the part drawn in the body ink, then its last few
  // characters, drawn faint so text emerges instead of appearing.
  readonly settled: string;
  readonly edge: string;
}

// How many characters at the growing edge are drawn faint. Wide enough
// that text emerges instead of appearing, narrow enough that the band a
// reader's eye sits on is not the shimmering one.
const EDGE = 10;

// The open tail split at its fading edge; a tail shorter than the edge
// is all edge.
export function faded(open: string): Pick<SayingLook, "settled" | "edge"> {
  return { settled: open.slice(0, -EDGE), edge: open.slice(-EDGE) };
}
