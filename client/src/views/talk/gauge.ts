// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the context ring round the coin key draws (client/Spec.lean §7J): how
// much of this session's window its latest turn used, and where the two
// reminders sound.
//
// **The pair of numbers is the city's own.** Used is the input tokens
// the provider reported for the latest turn and the window is the
// answering model's `context_tokens`, which is the pair the city's own
// reminder is computed from (glossary: context reminder), so the ring
// and the reminder a resident hears cannot disagree about how full the
// window is. The two percents are the room's `ConfigAnswer`, so neither
// is a number this page keeps.

import type { Belief } from "../../core/belief";
import { heldIn } from "../../core/belief/rooms";
import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import type { Address, Answer, EndpointSummary, RunId, Turn } from "../../wire";

// The percents at which the two reminders sound, or null where the page
// has not been told: a city that does not answer one gets no checkpoint
// for it rather than a copied number.
export interface Reminders {
  readonly first: number | null;
  readonly second: number | null;
}

export interface Context extends Reminders {
  readonly used: number;
  readonly window: number;
}

// The reminders a room's `Query::Config` answer states: the first from
// `kernel::consts_policy::CTX_REMINDER_FIRST_PERCENT`, the second as the
// room's settled threshold.
export function remindersOf(answer: Answer | undefined): Reminders {
  if (answer === undefined || !("config" in answer)) return { first: null, second: null };
  return { first: answer.config.first ?? null, second: answer.config.second.percent };
}

// The reminders as words, the ones the page was told and no others: the
// ring's tip and the session sheet's context cell say them alike.
export function remindersSaid(lang: Lang, reminders: Reminders): readonly string[] {
  return [
    ...(reminders.first === null ? [] : [fill(say(lang, "ring_first"), { n: String(reminders.first) })]),
    ...(reminders.second === null ? [] : [fill(say(lang, "ring_second"), { n: String(reminders.second) })]),
  ];
}

// The run of a room's newest session whose turns say how full its window
// is: runs the room held before that session began belong to the earlier
// stretch and are not measured.
export function sessionRunOf(belief: Belief, room: Address): RunId | undefined {
  const began = belief.sessions[room] ?? null;
  return heldIn(belief, room)
    .filter((each) => began === null || each.lastSeq > began)
    .at(-1)?.run;
}

// The context of a session whose turns are `turns`, or null when there
// is no window to measure against: a model whose endpoint stated no
// `context_tokens` gets no ring at all, because a fraction with no
// denominator is a number nobody should draw.
export function contextOf(
  turns: readonly Turn[],
  endpoints: readonly EndpointSummary[],
  reminders: Reminders,
): Context | null {
  const latest = turns.filter((turn) => turn.used !== undefined && turn.used !== null).at(-1);
  const model = turns.filter((turn) => typeof turn.model === "string").at(-1)?.model ?? null;
  if (model === null) return null;
  const window = endpoints
    .flatMap((endpoint) => endpoint.models)
    .find((row) => row.id === model)?.context_tokens;
  if (window === undefined || window === null || window <= 0) return null;
  return { used: latest?.used?.input ?? 0, window, ...reminders };
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
