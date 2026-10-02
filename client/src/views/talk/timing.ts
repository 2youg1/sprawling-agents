// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The figures a thread draws about time, and the one clock that moves
// the figures still running (client/Spec.lean §4-44).
//
// **Every figure is a difference of two moments the Ledger wrote, or
// nothing.** A call that answered took `answered - called`; a turn's
// time to first content is `first_at - t`, and its output rate is the
// provider's output count over `returned - first_at`. Either moment missing, a row
// the wire marks `unmeasured`, or a difference below zero is not a
// figure, and the reader draws no number rather than a guess. The one
// figure taken off this page's own clock is the running timer, which
// says how long a call has been going *so far*: it is recomputed from
// the call's own moment on every tick, never counted up from the ticks,
// so a tab hidden for a minute shows the right number the moment it is
// seen again.

import { readable } from "svelte/store";
import type { Readable } from "svelte/store";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Call, Turn } from "../../wire";

// How often a running timer is redrawn: a tenth of a second, the unit it
// is drawn in.
const TICK_MS = 100;

// A call shorter than this draws only its dot while it runs: under a
// second the person reads "working", and a figure flickering through
// tenths would be noise (Nielsen's 1 s).
const COUNTED_AFTER_MS = 1_000;

// Past this a running call says what it waits for, when that is known
// (Nielsen's 10 s).
export const NAMED_AFTER_MS = 10_000;

// How long a call took, as the Ledger measured it; `null` while it runs,
// when either moment is not a measurement, or when the two disagree.
// The one reading of a call's duration: every screen that draws one
// reads it here, so no two screens can say two numbers (client/Spec.lean §4-59).
export function lastedOf(call: Call): number | null {
  const answered = call.answered ?? null;
  if (answered === null || call.timing !== "measured") return null;
  const span = answered - call.called;
  return span < 0 ? null : span;
}

// The time from asking the model to its first content, or `null` when the
// turn carries no first moment or its opening moment is not a measurement.
export function ttftOf(turn: Turn): number | null {
  const first = turn.first_at ?? null;
  if (first === null || turn.timing !== "measured") return null;
  const span = first - turn.t;
  return span < 0 ? null : span;
}

// The reply's output rate in tokens a second: the provider's output
// count over the span from its first content to its return. `null` when
// either moment, the count or a positive span is missing - the count
// then does not belong to that span, and a guessed 0 would read as a
// stalled model (refrain §3-3).
export function tpsOf(turn: Turn): number | null {
  const first = turn.first_at ?? null;
  const returned = turn.returned ?? null;
  const output = turn.used?.output ?? 0;
  if (first === null || returned === null || output <= 0) return null;
  const span = returned - first;
  return span <= 0 ? null : (output * 1_000) / span;
}

// What a call's time cell says now.
export type CallTime =
  | { readonly kind: "landed"; readonly ms: number }
  | { readonly kind: "running"; readonly ms: number }
  // Running for less than `COUNTED_AFTER_MS`, or for a time this page's
  // clock cannot place (the call's moment lies ahead of it).
  | { readonly kind: "starting" }
  | { readonly kind: "unmeasured" };

export function callTime(call: Call, now: number): CallTime {
  switch (call.outcome) {
    case "waiting": {
      const ms = now - call.called;
      return ms < COUNTED_AFTER_MS || call.timing !== "measured" ? { kind: "starting" } : { kind: "running", ms };
    }
    case "answered":
    case "failed": {
      const ms = lastedOf(call);
      return ms === null ? { kind: "unmeasured" } : { kind: "landed", ms };
    }
  }
}

// A landed figure: whole milliseconds under a second, and seconds to the
// millisecond above it, because the Ledger's unit is the millisecond and
// a reader comparing `31 ms` with `3.412 s` should not convert. The one
// pair of units every landed duration is written in (client/Spec.lean §4-59).
export function landedWords(ms: number, lang: Lang): string {
  return ms < 1_000
    ? fill(say(lang, "talk_took_ms"), { n: String(Math.round(ms)) })
    : fill(say(lang, "talk_took_s"), { n: (ms / 1_000).toFixed(3) });
}

// A running figure, in the tenths the timer moves by.
export function runningWords(ms: number): string {
  return `${(Math.floor(ms / 100) / 10).toFixed(1)} s`;
}

// One ticker per clock, so every running timer on a page moves on the same
// tick and a page with fifty calls still runs one interval; the mailbox
// and the sessions pane read it too, so the page has one clock that
// moves (client/Spec.lean §4-59). It runs only
// while somebody is subscribed and the page is visible; becoming visible
// again publishes at once, from the clock.
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
