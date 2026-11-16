// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { AxError, DoctorAnswer } from "../../wire";
import { offerOf } from "./dependencies";

export type StepState = "waiting" | "running" | "done" | "failed";

export interface Step {
  readonly name: string;
  readonly state: StepState;

  readonly why: string | null;
}

export type Walk = readonly Step[];

export function plan(answer: DoctorAnswer): readonly string[] {
  return answer.items
    .filter((each) => each.tier === "develop" && offerOf(each) === "press")
    .map((each) => each.name);
}

export function started(names: readonly string[]): Walk {
  return advanced(names.map((name) => ({ name, state: "waiting", why: null })));
}

export function running(walk: Walk): string | null {
  return walk.find((each) => each.state === "running")?.name ?? null;
}

export function over(walk: Walk): boolean {
  return walk.every((each) => each.state === "done" || each.state === "failed");
}

export function answered(walk: Walk, answer: DoctorAnswer): Walk {
  const now = running(walk);
  const here = answer.items.some((each) => each.name === now && "present" in each.state);
  return here ? ended(walk, "done", null) : walk;
}

export function refused(walk: Walk, error: AxError): Walk {
  const now = running(walk);
  return now !== null && error.subject.startsWith(`${now}:`)
    ? ended(walk, "failed", error.recovery)
    : walk;
}

function ended(walk: Walk, state: "done" | "failed", why: string | null): Walk {
  return advanced(walk.map((each) => (each.state === "running" ? { ...each, state, why } : each)));
}

function advanced(walk: Walk): Walk {
  if (running(walk) !== null) return walk;
  const next = walk.findIndex((each) => each.state === "waiting");
  return walk.map((each, index) => (index === next ? { ...each, state: "running" } : each));
}
