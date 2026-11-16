// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";
import type { HTMLAttributes } from "svelte/elements";

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

export interface DecideProps {
  readonly kind: DecideKind;

  readonly asker: string;

  readonly at: string;

  readonly body: Snippet;

  readonly choices: readonly Choice[];
}

const GLYPH: Readonly<Record<DecideKind, GlyphName>> = {
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

export interface AnswerLook {
  readonly answer: Answer;
  readonly label: string;
  readonly tone: "primary" | "quiet";
  readonly why: string | undefined;
  readonly onPress: () => void;
  readonly marks: readonly string[];
}

export interface DecideWiring {
  readonly card: HTMLAttributes<HTMLDivElement>;
  readonly glyph: GlyphName;
  readonly asker: { readonly id: string; readonly text: string };
  readonly at: string;
  readonly answers: readonly AnswerLook[];
}

export interface DecideLook extends DecideWiring {
  readonly body: Snippet;
}

export interface Deciding {
  readonly uid: string;
  readonly hear: Attachment<HTMLDivElement>;

  readonly marksOf: (action: Action) => readonly string[];
}

const HEAR = createAttachmentKey();

export function lookOf(props: Omit<DecideProps, "body">, deciding: Deciding): DecideWiring {
  const asker = { id: `${deciding.uid}-asker`, text: props.asker };
  return {
    card: {
      role: "group",
      tabindex: -1,
      "aria-labelledby": asker.id,
      "data-decide": props.kind,
      "data-entry": "",
      [HEAR]: deciding.hear,
    },
    glyph: GLYPH[props.kind],
    asker,
    at: props.at,
    answers: ordered(props.choices).map((choice) => ({
      answer: choice.answer,
      label: choice.label,
      tone: choice.answer === "yes" ? "primary" : "quiet",
      why: choice.why,
      onPress: choice.onPress,
      marks: deciding.marksOf(KEY[choice.answer]),
    })),
  };
}
