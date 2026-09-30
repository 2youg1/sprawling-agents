// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whether this page watches the city's performance monitor, and the
// readings it has been sent (wire-SPEC 8-47).
//
// **The city samples only while somebody watches**, so this page says
// the most any of its watchers wants: `watch` while a monitor panel is
// open, `watch_summary` while only the fact bar's summary is, which the
// city answers by reading its own process alone, and `release` when
// neither is. It says it again on a new connection: the city forgets a
// session's watch when the socket closes.

import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { encodeFrame } from "./frames";
import type { Monitoring, Sample } from "../wire";

// The readings kept: one a second for five minutes, the length of the
// history `bin::monitor::CAPACITY` keeps on the city's side.
const KEPT = 300;

export interface Watching {
  readonly samples: Readable<readonly Sample[]>;
  // Starts watching the whole monitor; the returned function stops
  // this watcher.
  readonly watch: () => () => void;
  // Starts watching the summary alone; the returned function stops
  // this watcher.
  readonly watchSummary: () => () => void;
  readonly sampled: (sample: Sample) => void;
  readonly reconnected: () => void;
}

export function createWatching(sendText: (text: string) => boolean): Watching {
  const samples = writable<readonly Sample[]>([]);
  let pages = 0;
  let summaries = 0;
  let said: Monitoring = "release";

  function wanted(): Monitoring {
    if (pages > 0) return "watch";
    if (summaries > 0) return "watch_summary";
    return "release";
  }

  function settle(): void {
    const now = wanted();
    if (now === said) return;
    said = now;
    sendText(encodeFrame({ monitor: now }));
    if (now === "release") samples.set([]);
  }

  function watcher(count: (by: 1 | -1) => void): () => void {
    count(1);
    settle();
    let released = false;
    return () => {
      if (released) return;
      released = true;
      count(-1);
      settle();
    };
  }

  return {
    samples,
    watch: () =>
      watcher((by) => {
        pages += by;
      }),
    watchSummary: () =>
      watcher((by) => {
        summaries += by;
      }),
    sampled(sample) {
      samples.update((held) => [...held.slice(held.length >= KEPT ? 1 : 0), sample]);
    },
    reconnected() {
      if (said !== "release") sendText(encodeFrame({ monitor: said }));
    },
  };
}
