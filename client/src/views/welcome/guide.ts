// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where the first-run guide stands, read from two different sources
// that never stand in for each other (refrain §3-15, `crates/wire/
// Spec.lean` §8-68). **Whether a step is done** is the city's own
// configuration: a `main` model chosen, nothing missing on the doctor's
// use tier, a name written into the identity block. **What the person
// did with a step** - looked at it, put it off, where the guide reopens,
// whether they left it - is the guide's progress, which the city keeps
// per city. Opening a step is not doing it, and putting a step off never
// draws it as done.

import { GuideStep } from "../../wire";
import type { GuideMark, GuideProgress } from "../../wire";

export const STEPS: readonly GuideStep[] = GuideStep.literals;

// The four steps a person may put off. The first has no mark: only the
// city's saved endpoint and `main` model answer it.
export type Optional = Exclude<GuideStep, "provider">;

export const OPTIONAL: readonly Optional[] = STEPS.flatMap((step) => (step === "provider" ? [] : [step]));

// What the city's configuration says of each step: `true` done, `false`
// not done, `null` when this page has no reading that could say - the
// skills and MCP steps, whose completion is a per-building matter the
// guide does not judge.
export type Configured = Readonly<Record<GuideStep, boolean | null>>;

// How one step is drawn. `configured` outranks every mark, because a
// step the person skipped and then did elsewhere is done.
export type Standing = "configured" | "required" | "skipped" | "seen" | "untouched";

export function standingOf(step: GuideStep, progress: GuideProgress, configured: Configured): Standing {
  if (configured[step] === true) return "configured";
  if (step === "provider") return "required";
  return progress[step] ?? "untouched";
}

function markOf(progress: GuideProgress, step: Optional): GuideMark | null {
  return progress[step] ?? null;
}

// The step the guide opens on: where the person last was, or else the
// first step the city does not yet count as done and nobody has put off.
export function currentOf(progress: GuideProgress, configured: Configured): GuideStep {
  const at = progress.at ?? null;
  if (at !== null) return at;
  return STEPS.find((step) => {
    const standing = standingOf(step, progress, configured);
    return standing === "required" || standing === "untouched";
  }) ?? "provider";
}

// The step after this one, `null` after the last.
export function nextOf(step: GuideStep): GuideStep | null {
  return STEPS[STEPS.indexOf(step) + 1] ?? null;
}

// The person opened `step`: the guide reopens there, and an optional
// step nobody had marked is now seen. A step already put off stays put
// off, since looking at it again is not a change of mind.
export function opened(progress: GuideProgress, step: GuideStep): GuideProgress {
  if (step === "provider") return { ...progress, at: step };
  return { ...progress, at: step, [step]: markOf(progress, step) ?? "seen" };
}

// The person put `step` off: it is skipped, and the guide moves on.
export function putOff(progress: GuideProgress, step: Optional): GuideProgress {
  return { ...progress, [step]: "skipped", at: nextOf(step) ?? step };
}

// Every optional step nobody has looked at is put off at once.
export function putOffTheRest(progress: GuideProgress): GuideProgress {
  return OPTIONAL.reduce<GuideProgress>(
    (held, step) => (markOf(held, step) === null ? { ...held, [step]: "skipped" } : held),
    progress,
  );
}

// The person went to the conversation: opening the city no longer
// offers the guide, and the settings tree is the way back to it.
export function left(progress: GuideProgress): GuideProgress {
  return { ...progress, state: "left" };
}
