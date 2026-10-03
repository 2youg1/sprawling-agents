// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The page's timing of the key interactions (the roadmap's measuring
// standard, item 4): each one is a pair of User Timing marks and the
// measure between them, so a headless test, a real browser's
// performance panel and a harness that reads `performance` all read the
// same entries. This module takes no reading of its own and keeps no
// figure; whoever measures reads the entries.
//
// A duration is taken at whatever resolution the browser's
// `performance.now` gives: without cross-origin isolation browsers
// coarsen it (to 100 µs or more), with it to a few microseconds. Every
// measure carries which of the two it was taken under, so a reader never
// mistakes a coarsened figure for a fine one.

// The key interactions, each from the User's act to what answers it.
export type Interaction =
  | "keystroke_echo"
  | "send_shown"
  | "layer_drawn"
  | "panel_frame"
  | "session_switch"
  | "mailbox_open"
  | "settings_switch"
  | "scroll_frame";

export type Resolution = "isolated" | "coarsened";

// Every entry this module writes carries this prefix, so a reader can
// pick the page's interactions out of everything else on the timeline.
const PREFIX = "sprawling:";

// Times the two interactions the page sees at its root without a view
// taking part: a keystroke in a text box to the frame that shows it,
// and a scroll to the frame that draws it. The start is the event's own
// time stamp, so the wait before the handler ran is counted. Returns
// what stops the listening.
export function timeAtRoot(root: EventTarget): () => void {
  const typed = (event: Event): void => {
    markStart("keystroke_echo", event.timeStamp);
    markEndAtFrame("keystroke_echo");
  };
  const scrolled = (event: Event): void => {
    markStart("scroll_frame", event.timeStamp);
    markEndAtFrame("scroll_frame");
  };
  root.addEventListener("input", typed, { capture: true, passive: true });
  root.addEventListener("scroll", scrolled, { capture: true, passive: true });
  return () => {
    root.removeEventListener("input", typed, { capture: true });
    root.removeEventListener("scroll", scrolled, { capture: true });
  };
}

// Marks the User's act that opens `interaction`, now or at the time
// stamp of the event that carried it; a second start before the end
// restarts it, because only the latest act is the one answered.
export function markStart(interaction: Interaction, at: number = performance.now()): void {
  performance.mark(startOf(interaction), { startTime: at });
}

// How many measures of one interaction the timeline holds before the
// oldest batch is dropped: ten times the 1,000 a p999 needs, so a page
// open for days scrolling every frame does not grow without bound.
export const KEPT = 10_000;

const held = new Map<Interaction, number>();

// Marks what answers `interaction` and records the measure from its
// start. An end with no start open records nothing: something other
// than the User's act drew the answer. The `KEPT + 1`th measure starts
// the count again from itself.
export function markEnd(interaction: Interaction): void {
  const start = startOf(interaction);
  if (performance.getEntriesByName(start, "mark").length === 0) {
    return;
  }
  const count = held.get(interaction) ?? 0;
  if (count >= KEPT) {
    performance.clearMeasures(PREFIX + interaction);
  }
  performance.measure(PREFIX + interaction, { start, detail: { resolution: resolution() } });
  performance.clearMarks(start);
  held.set(interaction, count >= KEPT ? 1 : count + 1);
}

// Marks the end at the next animation frame, the moment the browser
// paints what the act changed; where no frame comes (a headless run),
// the end is marked now.
export function markEndAtFrame(interaction: Interaction): void {
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(() => {
      markEnd(interaction);
    });
    return;
  }
  markEnd(interaction);
}

// The durations recorded for `interaction`, oldest first, in
// milliseconds at the browser's resolution.
export function measured(interaction: Interaction): readonly number[] {
  return performance.getEntriesByName(PREFIX + interaction, "measure").map((entry) => entry.duration);
}

// Drops every entry of `interaction`, for a reader that has taken them.
export function forget(interaction: Interaction): void {
  performance.clearMarks(startOf(interaction));
  performance.clearMeasures(PREFIX + interaction);
  held.delete(interaction);
}

// Which resolution this page's clock runs at.
export function resolution(): Resolution {
  return globalThis.crossOriginIsolated ? "isolated" : "coarsened";
}

function startOf(interaction: Interaction): string {
  return `${PREFIX}${interaction}:start`;
}
