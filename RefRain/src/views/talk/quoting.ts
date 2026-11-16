// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { PreferenceDoor } from "../../core/prefs";

const boxes = new Map<string, (quote: string) => void>();

export function quoteInto(door: PreferenceDoor, at: string, quote: string): void {
  const box = boxes.get(at);
  if (box === undefined) door.setDraft(at, joined(door.draft(at), quote));
  else box(quote);
}


export function hearQuotes(at: string, take: (quote: string) => void): () => void {
  boxes.set(at, take);
  return () => {
    if (boxes.get(at) === take) boxes.delete(at);
  };
}


export function joined(words: string, quote: string): string {
  return words === "" ? quote : `${words}\n${quote}`;
}
