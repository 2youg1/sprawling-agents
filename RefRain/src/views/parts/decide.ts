// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../../core/snippet";
import type { ElementRef } from "../../core/element_ref";
import type { JSX } from "@solidjs/web";

import type { Action } from "../../core/keys";
import type { GlyphName } from "./glyph";

import { GLYPH, KEY, ordered, type Answer, type Choice, type DecideKind } from "./decide_model";
export { answering, KEY, ordered, type Answer, type Choice, type DecideKind } from "./decide_model";

export interface DecideProps {
  readonly kind: DecideKind;

  readonly asker: string;

  readonly at: string;

  readonly body: Snippet;

  readonly choices: readonly Choice[];
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
  readonly card: JSX.HTMLAttributes<HTMLDivElement> &
    Readonly<Record<symbol, ElementRef<HTMLDivElement>>>;
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
  readonly hear: ElementRef<HTMLDivElement>;

  readonly marksOf: (action: Action) => readonly string[];
}

const HEAR = Symbol();

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
