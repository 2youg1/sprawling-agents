// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { readable } from "svelte/store";
import type { Readable } from "svelte/store";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Call, Turn } from "../../wire";

const TICK_MS = 100;

const COUNTED_AFTER_MS = 1_000;

export const NAMED_AFTER_MS = 10_000;

export function lastedOf(call: Call): number | null {
  const answered = call.answered ?? null;
  if (answered === null || call.timing !== "measured") return null;
  const span = answered - call.called;
  return span < 0 ? null : span;
}

export function ttftOf(turn: Turn): number | null {
  const first = turn.first_at ?? null;
  if (first === null || turn.timing !== "measured") return null;
  const span = first - turn.t;
  return span < 0 ? null : span;
}

export function tpsOf(turn: Turn): number | null {
  const first = turn.first_at ?? null;
  const returned = turn.returned ?? null;
  const output = turn.used?.output ?? 0;
  if (first === null || returned === null || output <= 0) return null;
  const span = returned - first;
  return span <= 0 ? null : (output * 1_000) / span;
}

export type Took =
  { readonly unit: "us"; readonly n: number } | { readonly unit: "ms"; readonly n: number };

export function tookOf(call: Call): Took | null {
  const us = call.took_us ?? null;
  if (us !== null && us >= 0 && call.outcome !== "waiting") return { unit: "us", n: us };
  const ms = lastedOf(call);
  return ms === null ? null : { unit: "ms", n: ms };
}

export function ttftTookOf(turn: Turn): Took | null {
  const us = turn.used?.first_us ?? null;
  if (us !== null && us >= 0) return { unit: "us", n: us };
  const ms = ttftOf(turn);
  return ms === null ? null : { unit: "ms", n: ms };
}

export type CallTime =
  | { readonly kind: "landed"; readonly took: Took }
  | { readonly kind: "running"; readonly ms: number }

  | { readonly kind: "starting" }
  | { readonly kind: "unmeasured" };

export function callTime(call: Call, now: number): CallTime {
  switch (call.outcome) {
    case "waiting": {
      const ms = now - call.called;
      return ms < COUNTED_AFTER_MS || call.timing !== "measured"
        ? { kind: "starting" }
        : { kind: "running", ms };
    }
    case "answered":
    case "failed": {
      const took = tookOf(call);
      return took === null ? { kind: "unmeasured" } : { kind: "landed", took };
    }
  }
}

const MICROS_UNDER_US = 10_000;

export function tookWords(took: Took, lang: Lang): string {
  switch (took.unit) {
    case "us":
      return took.n < MICROS_UNDER_US
        ? fill(say(lang, "talk_took_us"), { n: String(Math.round(took.n)) })
        : msWords(took.n / 1_000, lang);
    case "ms":
      return msWords(took.n, lang);
  }
}

function msWords(ms: number, lang: Lang): string {
  return ms < 1_000
    ? fill(say(lang, "talk_took_ms"), { n: String(Math.round(ms)) })
    : fill(say(lang, "talk_took_s"), { n: (ms / 1_000).toFixed(3) });
}

export function runningWords(ms: number): string {
  return `${(Math.floor(ms / 100) / 10).toFixed(1)} s`;
}

const tickers = new WeakMap<() => number, Readable<number>>();

export function ticker(now: () => number): Readable<number> {
  const held = tickers.get(now);
  if (held !== undefined) return held;
  const made = readable(now(), (set) => {
    let interval: ReturnType<typeof setInterval> | null = null;
    const start = (): void => {
      set(now());
      interval ??= setInterval(() => {
        set(now());
      }, TICK_MS);
    };
    const stop = (): void => {
      if (interval !== null) clearInterval(interval);
      interval = null;
    };
    const seen = (): void => {
      if (document.visibilityState === "hidden") stop();
      else start();
    };
    seen();
    document.addEventListener("visibilitychange", seen);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", seen);
    };
  });
  tickers.set(now, made);
  return made;
}
