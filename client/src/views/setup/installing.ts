// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One press that installs everything the develop tier is missing: which
// items it runs, and how far it has got.
//
// The page walks the list one install at a time, in the city's table
// order, because a later row's recipe starts a program an earlier row
// installs (`crates/sprawling/Spec.lean` §8-58). An install ends in exactly one of two
// ways (§8-64): a fresh answer about this machine in which the item is
// present, or a refusal whose subject starts with the item's name. The
// walk moves on at either, so it never reads the wording of a log line.

import type { AxError, DoctorAnswer } from "../../wire";
import { offerOf } from "./dependencies";

export type StepState = "waiting" | "running" | "done" | "failed";

export interface Step {
  readonly name: string;
  readonly state: StepState;
  // What the city said when the install failed: its recovery names the
  // file the installer's output went to.
  readonly why: string | null;
}

export type Walk = readonly Step[];

// What one press installs: every absent develop item whose recipe this
// city may run. A printed script and a manual step stay with the person.
export function plan(answer: DoctorAnswer): readonly string[] {
  return answer.items
    .filter((each) => each.tier === "develop" && offerOf(each) === "press")
    .map((each) => each.name);
}

export function started(names: readonly string[]): Walk {
  return advanced(names.map((name) => ({ name, state: "waiting", why: null })));
}

// The item being installed now, if any.
export function running(walk: Walk): string | null {
  return walk.find((each) => each.state === "running")?.name ?? null;
}

export function over(walk: Walk): boolean {
  return walk.every((each) => each.state === "done" || each.state === "failed");
}

// A fresh answer ends the running install when the item is now here.
export function answered(walk: Walk, answer: DoctorAnswer): Walk {
  const now = running(walk);
  const here = answer.items.some((each) => each.name === now && "present" in each.state);
  return here ? ended(walk, "done", null) : walk;
}

// A refusal naming the running item ends it as failed.
export function refused(walk: Walk, error: AxError): Walk {
  const now = running(walk);
  return now !== null && error.subject.startsWith(`${now}:`) ? ended(walk, "failed", error.recovery) : walk;
}

function ended(walk: Walk, state: "done" | "failed", why: string | null): Walk {
  return advanced(walk.map((each) => (each.state === "running" ? { ...each, state, why } : each)));
}

// The first waiting item starts once nothing else is running.
function advanced(walk: Walk): Walk {
  if (running(walk) !== null) return walk;
  const next = walk.findIndex((each) => each.state === "waiting");
  return walk.map((each, index) => (index === next ? { ...each, state: "running" } : each));
}
