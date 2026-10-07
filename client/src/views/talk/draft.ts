// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the words a person typed and did not send are kept, one place
// at a time (client/Spec.lean §4-63): the composer reads them when it
// arrives at a place and writes them back as they change.

import type { PreferenceDoor } from "../../core/prefs";

// How long a keystroke waits before the draft is written.
const DRAFT_MS = 500;

// What was typed and not sent, for one place a person writes. The words
// not yet written and the timer that will write them travel with the
// door they go through, so the box cannot end up with one of the three
// and not the others.
//
// A keystroke schedules a write instead of making one: the draft door
// touches browser storage, and a write per key is a write per key. A
// change that empties or replaces the box writes at once - those are
// rare, and a reload right after a send must not resurrect the words
// that just went out.
export interface Draft {
  // Read once, when the box mounts: the door is a plain function, not a
  // signal.
  readonly read: string;
  // Words typed, to be written when the person pauses.
  keep(words: string): void;
  // Words placed in the box - sent away, transcribed, completed - to be
  // written now.
  replace(words: string): void;
  flush(): void;
}

export function draftAt(door: PreferenceDoor, at: string | undefined): Draft {
  let pending: string | null = null;
  let flushing: ReturnType<typeof setTimeout> | undefined = undefined;

  function flush(): void {
    if (flushing !== undefined) {
      clearTimeout(flushing);
      flushing = undefined;
    }
    if (pending === null || at === undefined) return;
    door.setDraft(at, pending);
    pending = null;
  }

  return {
    read: at === undefined ? "" : door.draft(at),
    keep(words) {
      pending = words;
      if (flushing !== undefined) clearTimeout(flushing);
      flushing = setTimeout(flush, DRAFT_MS);
    },
    replace(words) {
      pending = words;
      flush();
    },
    flush,
  };
}
