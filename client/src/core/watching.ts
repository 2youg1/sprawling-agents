// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Whether this page watches the city's performance monitor, and the
// readings it has been sent (channels-SPEC 8-47).
//
// **The city samples only while somebody watches**, so this page says
// `watch` when its first panel opens and `release` when its last one
// closes, and says `watch` again on a new connection: the city forgets
// a session's watch when the socket closes.

import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import { encodeFrame } from "./frames";
import type { Sample } from "../wire";

// The readings kept: one a second for five minutes, the length of the
// history `bin::monitor::CAPACITY` keeps on the city's side.
const KEPT = 300;

export interface Watching {
  readonly samples: Readable<readonly Sample[]>;
  // Starts watching; the returned function stops this watcher.
  readonly watch: () => () => void;
  readonly sampled: (sample: Sample) => void;
  readonly reconnected: () => void;
}

export function createWatching(sendText: (text: string) => boolean): Watching {
  const samples = writable<readonly Sample[]>([]);
  let watchers = 0;

  function say(monitor: "watch" | "release"): void {
    sendText(encodeFrame({ monitor }));
  }

  return {
    samples,
    watch() {
      watchers += 1;
      if (watchers === 1) say("watch");
      let released = false;
      return () => {
        if (released) return;
        released = true;
        watchers -= 1;
        if (watchers > 0) return;
        say("release");
        samples.set([]);
      };
    },
    sampled(sample) {
      samples.update((held) => [...held.slice(held.length >= KEPT ? 1 : 0), sample]);
    },
    reconnected() {
      if (watchers > 0) say("watch");
    },
  };
}
