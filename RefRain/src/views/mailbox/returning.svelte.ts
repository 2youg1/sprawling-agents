// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The way back from a letter to the mailbox row that opened it (client
// D73): the letter's closing names the row, and `mailbox.svelte` - the
// one owner of whether the column is shown - takes it from here, opens
// the column through `stepLetter` and puts the focus on the row with
// that card's `data-letter`.

let wanted = $state<string | null>(null);

export function returnTo(row: string): void {
  wanted = row;
}

export function wantedRow(): string | null {
  return wanted;
}

export function takeRow(): void {
  wanted = null;
}
