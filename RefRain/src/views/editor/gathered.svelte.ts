// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { untrack } from "svelte";

import { readAnswer } from "../../core/answered";
import { joined, nextSpan } from "../../core/document_windows";
import type { Gathering } from "../../core/document_windows";
import type { Asking } from "../../core/asking";

export class Gathered {
  value = $state<Gathering | null>(null);
  lost = $state<string | null>(null);

  constructor(asking: Asking) {
    $effect(() => {
      const at = this.value;
      const span = at === null || this.lost !== null ? null : nextSpan(at);
      if (at === null || span === null) return;
      return asking.ask({ range: { version: at.version, range: span } }).subscribe((answer) => {
        const read = readAnswer(answer, (held) => ("range" in held ? held.range : undefined));
        untrack(() => {
          if (read.kind === "held" && this.value !== null)
            this.value = joined(this.value, read.value);
          else if (read.kind === "unavailable") this.lost = read.query;
        });
      });
    });
  }

  start(from: Gathering | null): void {
    this.value = from;
    this.lost = null;
  }
}
