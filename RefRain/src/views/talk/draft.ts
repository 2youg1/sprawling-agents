// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { PreferenceDoor } from "../../core/prefs";
import type { Address, Seq } from "../../wire";


export function draftPlace(room: Address, began: Seq | null): string {
  return began === null ? room : `${room}:${String(began)}`;
}


const DRAFT_MS = 500;


export interface Draft {
  
  
  readonly read: string;
  
  keep(words: string): void;
  
  
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
