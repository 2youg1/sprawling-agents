// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A line quoted into the conversation at a place (client/Spec.lean §7-2,
// 4-63). When a box is open there, the quote is handed to it, and the
// box joins it to the words being typed without moving the caret; only
// when none is open does it join that place's draft row, which the next
// box to open there reads. A box that is open is never written to
// through the row, because the keys it has not flushed yet would be
// overwritten by the older words the row still holds.

import type { PreferenceDoor } from "../../core/prefs";

const boxes = new Map<string, (quote: string) => void>();

export function quoteInto(door: PreferenceDoor, at: string, quote: string): void {
  const box = boxes.get(at);
  if (box === undefined) door.setDraft(at, joined(door.draft(at), quote));
  else box(quote);
}

// The box open at a place listens for quotes until the returned hand
// is called; a second box at the same place takes over from the first.
export function hearQuotes(at: string, take: (quote: string) => void): () => void {
  boxes.set(at, take);
  return () => {
    if (boxes.get(at) === take) boxes.delete(at);
  };
}

// A quote on a line of its own after whatever was already written.
export function joined(words: string, quote: string): string {
  return words === "" ? quote : `${words}\n${quote}`;
}
