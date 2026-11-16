// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

const PREFIX = "sprawling:";

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

export function markStart(interaction: Interaction, at: number = performance.now()): void {
  performance.mark(startOf(interaction), { startTime: at });
}

export const KEPT = 10_000;

const held = new Map<Interaction, number>();

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

export function markEndAtFrame(interaction: Interaction): void {
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(() => {
      markEnd(interaction);
    });
    return;
  }
  markEnd(interaction);
}

export function measured(interaction: Interaction): readonly number[] {
  return performance
    .getEntriesByName(PREFIX + interaction, "measure")
    .map((entry) => entry.duration);
}

export function forget(interaction: Interaction): void {
  performance.clearMarks(startOf(interaction));
  performance.clearMeasures(PREFIX + interaction);
  held.delete(interaction);
}

export function resolution(): Resolution {
  return globalThis.crossOriginIsolated ? "isolated" : "coarsened";
}

function startOf(interaction: Interaction): string {
  return `${PREFIX}${interaction}:start`;
}
