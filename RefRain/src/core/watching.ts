// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { writable } from "./store";
import type { Readable } from "./store";

import { encodeFrame } from "./frames";
import { BeatMs } from "../wire";
import type { Monitoring, Sample } from "../wire";

const KEPT = 300;

export interface Watching {
  readonly samples: Readable<readonly Sample[]>;

  readonly watch: () => () => void;

  readonly watchSummary: () => () => void;
  readonly sampled: (sample: Sample) => void;
  readonly reconnected: () => void;

  readonly setBeat: (ms: number) => void;
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
    setBeat(ms) {
      sendText(encodeFrame({ monitor: { beat: BeatMs.make(ms) } }));
    },
  };
}
