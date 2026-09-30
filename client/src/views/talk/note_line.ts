// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Note } from "../../wire";

// One note's own line in the Ledger, which is what a list of notes is
// keyed on: every variant carries one, and `note_line.svelte` draws each.
export function noteAt(note: Note): number {
  if ("arrived" in note) return note.arrived.at;
  if ("refused" in note) return note.refused.at;
  if ("checkpointed" in note) return note.checkpointed.at;
  if ("waiting" in note) return note.waiting.at;
  if ("unreadable" in note) return note.unreadable.at;
  return note.discarded.at;
}
