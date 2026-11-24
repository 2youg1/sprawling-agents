// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Action } from "../../core/keys";
import type { GlyphName } from "./glyph";

export type DecideKind = "question" | "ask" | "proposal";

export type Answer = "yes" | "edit" | "no";

export interface Choice {
  readonly answer: Answer;
  readonly label: string;

  readonly why?: string | undefined;
  readonly onPress: () => void;
}

export const GLYPH: Readonly<Record<DecideKind, GlyphName>> = {
  question: "hand",
  ask: "gate",
  proposal: "propose",
};

export const KEY: Readonly<Record<Answer, Action>> = {
  yes: "decide.yes",
  edit: "decide.edit",
  no: "decide.no",
};

const ORDER: readonly Answer[] = ["yes", "edit", "no"];

export function ordered(choices: readonly Choice[]): readonly Choice[] {
  return ORDER.flatMap((answer) =>
    choices.filter((choice) => choice.answer === answer).slice(0, 1),
  );
}

export function answering(offered: readonly Choice[], action: Action | null): Choice | undefined {
  const chosen = offered.find((choice) => KEY[choice.answer] === action);
  return chosen === undefined || chosen.why !== undefined ? undefined : chosen;
}
