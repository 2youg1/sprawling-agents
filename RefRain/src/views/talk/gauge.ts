// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Belief } from "../../core/belief";
import { heldIn } from "../../core/belief/rooms";
import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { kilo } from "../../core/time";
import type { Address, Answer, RunId } from "../../wire";


export interface ContextTurn {
  readonly model?: string | null | undefined;
  readonly used?: { readonly input: number } | null | undefined;
}


export interface ContextModel {
  readonly id: string;
  readonly context_tokens?: number | null | undefined;
}


export interface ContextEndpoint {
  readonly models: readonly ContextModel[];
}


export interface Reminders {
  readonly first: number | null;
  readonly second: number | null;
}

export interface Context extends Reminders {
  readonly used: number;
  readonly window: number;
}


export function remindersOf(answer: Answer | undefined): Reminders {
  if (answer === undefined || !("config" in answer)) return { first: null, second: null };
  return { first: answer.config.first ?? null, second: answer.config.second.percent };
}


export function lockOfConfig(answer: Answer | undefined): number | null {
  if (answer === undefined || !("config" in answer)) return null;
  const lock = answer.config.context_lock_tokens;
  return lock === undefined || lock === null || lock <= 0 ? null : lock;
}


export function remindersSaid(lang: Lang, reminders: Reminders): readonly string[] {
  return [
    ...(reminders.first === null
      ? []
      : [fill(say(lang, "ring_first"), { n: String(reminders.first) })]),
    ...(reminders.second === null
      ? []
      : [fill(say(lang, "ring_second"), { n: String(reminders.second) })]),
  ];
}


export function sessionRunOf(belief: Belief, room: Address): RunId | undefined {
  const began = belief.sessions[room] ?? null;
  return heldIn(belief, room)
    .filter((each) => began === null || each.lastSeq > began)
    .at(-1)?.run;
}


export function contextOf(
  turns: readonly ContextTurn[],
  endpoints: readonly ContextEndpoint[],
  reminders: Reminders,
  lockTokens: number | null = null,
): Context | null {
  const latest = turns.filter((turn) => turn.used !== undefined && turn.used !== null).at(-1);
  const model = turns.filter((turn) => typeof turn.model === "string").at(-1)?.model ?? null;
  if (model === null) return null;
  const modelWindow = endpoints
    .flatMap((endpoint) => endpoint.models)
    .find((row) => row.id === model)?.context_tokens;
  if (modelWindow === undefined || modelWindow === null || modelWindow <= 0) return null;
  const window =
    lockTokens !== null && lockTokens > 0 ? Math.min(modelWindow, lockTokens) : modelWindow;
  return { used: latest?.used?.input ?? 0, window, ...reminders };
}


export function usedPercent(context: Context): number {
  return Math.min(100, Math.max(0, Math.round((context.used / context.window) * 100)));
}


export function remainingArc(context: Context): Arc {
  const used = usedPercent(context);
  return { dasharray: `${String(100 - used)} 100`, dashoffset: String(-used) };
}


export type Reminder = "first" | "second";

export interface Checkpoint {
  readonly reminder: Reminder;
  
  readonly at: number;
}


export function checkpointsOf(reminders: Reminders): readonly Checkpoint[] {
  return [
    ...(reminders.first === null ? [] : [{ reminder: "first" as const, at: reminders.first }]),
    ...(reminders.second === null ? [] : [{ reminder: "second" as const, at: reminders.second }]),
  ];
}


export interface MeterWire {
  readonly role: "button";
  readonly type: "button";
  readonly tabindex: 0;
  readonly "aria-label": string;
  readonly "aria-valuemin": 0;
  readonly "aria-valuemax": number;
  readonly "aria-valuenow": number;
  readonly "aria-valuetext": string;
}


export interface RingLook {
  readonly meter: MeterWire;
  
  readonly reading: string;
  readonly arc: Arc;
  readonly checkpoints: readonly Checkpoint[];
}

export interface Arc {
  readonly dasharray: string;
  readonly dashoffset: string;
}


export function ringOf(lang: Lang, context: Context | null): RingLook | null {
  if (context === null) return null;
  const reading = [
    fill(say(lang, "ring_used"), { used: kilo(context.used), window: kilo(context.window) }),
    fill(say(lang, "ring_share"), { n: String(usedPercent(context)) }),
    ...remindersSaid(lang, context),
  ].join(" · ");
  return {
    meter: {
      role: "button",
      type: "button",
      tabindex: 0,
      "aria-label": say(lang, "context_report_open"),
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


export interface ContextBarLook {
  readonly share: number;
  readonly handoff: Checkpoint | undefined;
  readonly said: string;
}


export function barOf(lang: Lang, context: Context | null): ContextBarLook | null {
  if (context === null) return null;
  const share = usedPercent(context);
  return {
    share,
    handoff: checkpointsOf(context).find((each) => each.reminder === "second"),
    said: fill(say(lang, "ring_share"), { n: String(share) }),
  };
}
