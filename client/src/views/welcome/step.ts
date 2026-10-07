// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the head of one step of the guide is handed (`step.look.svelte`):
// its number, its title, the word for where it stands, and the
// disclosure button's wiring. Which step is open and what a press does
// are `welcome.svelte`'s; the words are decided here, once.

import type { Key } from "../../core/lang";
import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { GuideStep } from "../../wire";
import type { Standing } from "./guide";

export interface StepWire {
  readonly id: string;
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly onclick: () => void;
}

export interface StepLook {
  // Two digits, so the titles stand on one line whatever the count.
  readonly number: string;
  readonly title: string;
  readonly standing: Standing;
  // The word for the standing; a step nobody touched says nothing.
  readonly word: string | undefined;
  readonly wire: StepWire;
}

export interface StepState {
  readonly step: GuideStep;
  // Its place in the guide, from 0.
  readonly at: number;
  readonly standing: Standing;
  readonly open: boolean;
  // The id of the head; the body is `${id}-body`.
  readonly id: string;
  readonly lang: Lang;
}

const TITLE: Readonly<Record<GuideStep, Key>> = {
  provider: "guide_step_provider",
  dependencies: "guide_step_dependencies",
  texts: "guide_step_texts",
  skills: "guide_step_skills",
  mcp: "guide_step_mcp",
};

// What a step is for, drawn at the top of its body.
export const ABOUT: Readonly<Record<GuideStep, Key>> = {
  provider: "guide_step_provider_about",
  dependencies: "guide_step_dependencies_about",
  texts: "guide_step_texts_about",
  skills: "guide_step_skills_about",
  mcp: "guide_step_mcp_about",
};

const WORD: Readonly<Record<Standing, Key | null>> = {
  configured: "guide_standing_configured",
  required: "guide_standing_required",
  skipped: "guide_standing_skipped",
  seen: "guide_standing_seen",
  untouched: null,
};

export function bodyId(id: string): string {
  return `${id}-body`;
}

export function stepLookOf(state: StepState, toggle: () => void): StepLook {
  const word = WORD[state.standing];
  return {
    number: String(state.at + 1).padStart(2, "0"),
    title: say(state.lang, TITLE[state.step]),
    standing: state.standing,
    word: word === null ? undefined : say(state.lang, word),
    wire: {
      id: state.id,
      type: "button",
      "aria-expanded": state.open,
      "aria-controls": bodyId(state.id),
      onclick: toggle,
    },
  };
}
