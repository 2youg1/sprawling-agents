// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { GuideStep } from "../../wire";
import type { EndpointsAnswer, GuideMark, GuideProgress } from "../../wire";

export const STEPS: readonly GuideStep[] = GuideStep.literals;

export type Optional = Exclude<GuideStep, "provider">;

const OPTIONAL: readonly Optional[] = STEPS.flatMap((step) => (step === "provider" ? [] : [step]));

export type Configured = Readonly<Record<GuideStep, boolean | null>>;

export type Standing = "configured" | "required" | "skipped" | "seen" | "untouched";

export function standingOf(
  step: GuideStep,
  progress: GuideProgress,
  configured: Configured,
): Standing {
  if (configured[step] === true) return "configured";
  if (step === "provider") return "required";
  return progress[step] ?? "untouched";
}

function markOf(progress: GuideProgress, step: Optional): GuideMark | null {
  return progress[step] ?? null;
}

export function currentOf(progress: GuideProgress, configured: Configured): GuideStep {
  const at = progress.at ?? null;
  if (at !== null) return at;
  return (
    STEPS.find((step) => {
      const standing = standingOf(step, progress, configured);
      return standing === "required" || standing === "untouched";
    }) ?? "provider"
  );
}

function nextOf(step: GuideStep): GuideStep | null {
  return STEPS[STEPS.indexOf(step) + 1] ?? null;
}

export function opened(progress: GuideProgress, step: GuideStep): GuideProgress {
  if (step === "provider") return { ...progress, at: step };
  return { ...progress, at: step, [step]: markOf(progress, step) ?? "seen" };
}

export function putOff(progress: GuideProgress, step: Optional): GuideProgress {
  return { ...progress, [step]: "skipped", at: nextOf(step) ?? step };
}

export function putOffTheRest(progress: GuideProgress): GuideProgress {
  return OPTIONAL.reduce<GuideProgress>(
    (held, step) => (markOf(held, step) === null ? { ...held, [step]: "skipped" } : held),
    progress,
  );
}

export function left(progress: GuideProgress): GuideProgress {
  return { ...progress, state: "left" };
}

export function skipAll(progress: GuideProgress): GuideProgress {
  return left(putOffTheRest(progress));
}

export function advanced(
  progress: GuideProgress,
  open: GuideStep,
  before: Configured,
  after: Configured,
): GuideProgress | null {
  if (before[open] !== false || after[open] !== true) return null;
  const next = STEPS.slice(STEPS.indexOf(open) + 1).find((step) => {
    const standing = standingOf(step, progress, after);
    return standing !== "configured" && standing !== "skipped";
  });
  return next === undefined ? null : opened(progress, next);
}

export interface Launch {
  readonly endpoints: number;
  readonly main: boolean;
  readonly welcomed: boolean;
}

export function launchOf(answer: EndpointsAnswer, welcomed: boolean): Launch {
  return {
    endpoints: answer.endpoints.length,
    main: answer.chosen.some((each) => each.tag === "main"),
    welcomed,
  };
}

export function opensGuide(launch: Launch): boolean {
  return launch.endpoints === 0 || (!launch.main && !launch.welcomed);
}
