// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the context ring round the coin key draws (client-SPEC 7J): how
// much of this session's window its latest turn used, and where the two
// reminders sound.
//
// **The pair of numbers is the city's own.** Used is the input tokens
// the provider reported for the latest turn and the window is the
// answering model's `context_tokens`, which is the pair the city's own
// reminder is computed from (glossary: context reminder), so the ring
// and the reminder a resident hears cannot disagree about how full the
// window is.

import type { EndpointSummary, Turn } from "../../wire";

export interface Context {
  readonly used: number;
  readonly window: number;
  // The percents at which the two reminders sound, or null where the
  // page has not been told: the first is not on the wire yet, so the
  // ring draws no green checkpoint rather than a copied number.
  readonly first: number | null;
  readonly second: number | null;
}

// The context of a session whose turns are `turns`, or null when there
// is no window to measure against: a model whose endpoint stated no
// `context_tokens` gets no ring at all, because a fraction with no
// denominator is a number nobody should draw.
export function contextOf(
  turns: readonly Turn[],
  endpoints: readonly EndpointSummary[],
  second: number | null,
): Context | null {
  const latest = turns.filter((turn) => turn.used !== undefined && turn.used !== null).at(-1);
  const model = turns.filter((turn) => typeof turn.model === "string").at(-1)?.model ?? null;
  if (model === null) return null;
  const window = endpoints
    .flatMap((endpoint) => endpoint.models)
    .find((row) => row.id === model)?.context_tokens;
  if (window === undefined || window === null || window <= 0) return null;
  return { used: latest?.used?.input ?? 0, window, first: null, second };
}

// The share of the window used, as a whole percent clamped to the ring.
export function usedPercent(context: Context): number {
  return Math.min(100, Math.max(0, Math.round((context.used / context.window) * 100)));
}

// The remaining arc, for a circle drawn with `pathLength="100"` and
// turned so that it starts at twelve o'clock: the used part is eaten
// from the start clockwise, so what is drawn begins where the used part
// ends and runs to the start. Only what remains is stroked; an overlay
// of the page colour over a whole ring leaves a fringe where they meet.
export function remainingArc(context: Context): { readonly dasharray: string; readonly dashoffset: string } {
  const used = usedPercent(context);
  return { dasharray: `${String(100 - used)} 100`, dashoffset: String(-used) };
}
