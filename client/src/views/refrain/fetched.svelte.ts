// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One stored object's bytes as they arrive: the page asks the next
// window (`core/document_bytes.ts`) until the bytes meet the object's
// end, or the city says it no longer holds the object (client/Spec.lean
// §4-61). Made while a component initialises, so the asking stops with
// the component.

import { untrack } from "svelte";

import { readAnswer } from "../../core/answered";
import type { Asking } from "../../core/asking";
import { fetchedOf, fetching, joinedBytes, nextBytes } from "../../core/document_bytes";
import type { Fetched, Fetching } from "../../core/document_bytes";
import type { B3Hash } from "../../wire";

export class StoredBytes {
  // The question the city could not answer, when it could not.
  lost = $state<string | null>(null);
  private at = $state.raw<Fetching | null>(null);

  readonly value = $derived<Fetched | null>(this.at === null ? null : fetchedOf(this.at));

  constructor(asking: Asking) {
    $effect(() => {
      const at = this.at;
      const span = at === null || this.lost !== null ? null : nextBytes(at);
      if (at === null || span === null) return;
      return asking.ask({ bytes: { version: at.version, range: span } }).subscribe((answer) => {
        const read = readAnswer(answer, (held) => ("bytes" in held ? held.bytes : undefined));
        untrack(() => {
          if (read.kind === "held" && this.at !== null) this.at = joinedBytes(this.at, read.value);
          else if (read.kind === "unavailable") this.lost = read.query;
        });
      });
    });
  }

  start(version: B3Hash | null): void {
    if (version === this.at?.version) return;
    this.at = version === null ? null : fetching(version);
    this.lost = null;
  }
}
