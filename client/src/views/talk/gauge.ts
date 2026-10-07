// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the context ring round the coin key draws (docs/frontend-method.md §7J): how
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
import { kilo } from "../../core/time";
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
export function remainingArc(context: Context): Arc {
  const used = usedPercent(context);
  return { dasharray: `${String(100 - used)} 100`, dashoffset: String(-used) };
}

// Which reminder a checkpoint marks: the first, or the second, which is
// the handoff. The two are drawn alike and told apart by colour and by
// where they sit, so one look draws both (`checkpoint.look.svelte`).
export type Reminder = "first" | "second";

export interface Checkpoint {
  readonly reminder: Reminder;
  // Its percent of the window, the place it is drawn at.
  readonly at: number;
}

// The checkpoints a room's reminders put on the ring, the ones the page
// was told and no others.
export function checkpointsOf(reminders: Reminders): readonly Checkpoint[] {
  return [
    ...(reminders.first === null ? [] : [{ reminder: "first" as const, at: reminders.first }]),
    ...(reminders.second === null ? [] : [{ reminder: "second" as const, at: reminders.second }]),
  ];
}

// What the ring's focusable element carries, spread onto it as it is.
// It is a meter rather than a button: it reports a reading and does
// nothing when pressed. It takes focus so a keyboard reaches the reading
// the tip spells out, as a pointer does by hovering; the tip says the
// same words as `aria-valuetext`, so the meter is not also described by
// it, which would have a screen reader read the line twice.
export interface MeterWire {
  readonly role: "meter";
  readonly tabindex: 0;
  readonly "aria-label": string;
  readonly "aria-valuemin": 0;
  readonly "aria-valuemax": number;
  readonly "aria-valuenow": number;
  readonly "aria-valuetext": string;
}

// Everything the ring's look is given. The coin key it surrounds is the
// look's child, not part of this value, because the ring draws round
// whatever key the composer puts in it.
export interface RingLook {
  readonly meter: MeterWire;
  // The one line the ring says on hover and on focus.
  readonly reading: string;
  readonly arc: Arc;
  readonly checkpoints: readonly Checkpoint[];
}

export interface Arc {
  readonly dasharray: string;
  readonly dashoffset: string;
}

// The ring for a session's context, or null where there is no window to
// draw against and the coin key stands alone. The reading is used
// against the window, the share, and the reminders the page knows
// about, which is also the whole of what a screen reader is told.
export function ringOf(lang: Lang, context: Context | null): RingLook | null {
  if (context === null) return null;
  const reading = [
    fill(say(lang, "ring_used"), { used: kilo(context.used), window: kilo(context.window) }),
    fill(say(lang, "ring_share"), { n: String(usedPercent(context)) }),
    ...remindersSaid(lang, context),
  ].join(" · ");
  return {
    meter: {
      role: "meter",
      tabindex: 0,
      "aria-label": say(lang, "ring_name"),
      "aria-valuemin": 0,
      "aria-valuemax": context.window,
      "aria-valuenow": Math.min(context.used, context.window),
      "aria-valuetext": reading,
    },
    reading,
    arc: remainingArc(context),
    checkpoints: checkpointsOf(context),
  };
}

// Everything a sessions row's context bar is given: the share of the
// window used, the handoff checkpoint where the room states one, and
// the share in words for a screen reader, since the bar itself is
// drawing. Only the handoff is marked, because the bar answers which
// session is about to hand off; the ring beside the composer carries
// both reminders.
export interface ContextBarLook {
  readonly share: number;
  readonly handoff: Checkpoint | undefined;
  readonly said: string;
}

// The bar for a session's context, or null where its model stated no
// window, as the ring draws no ring.
export function barOf(lang: Lang, context: Context | null): ContextBarLook | null {
  if (context === null) return null;
  const share = usedPercent(context);
  return {
    share,
    handoff: checkpointsOf(context).find((each) => each.reminder === "second"),
    said: fill(say(lang, "ring_share"), { n: String(share) }),
  };
}
