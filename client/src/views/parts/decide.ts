// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one card for everything that stops and asks the person
// (client/Spec.lean §7C), as its look is handed it, and the rule by
// which a chord answers it (client/spec/Views/Parts/Decide.lean). No
// DOM and no runes: the seat reads the key table and hears the keys.

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";
import type { HTMLAttributes } from "svelte/elements";

import type { Action } from "../../core/keys";
import type { GlyphName } from "./glyph";

// What is being asked. `question` is a design question a resident
// filed (`ApprovalItem`); `ask` is a door waiting on the person's own
// hand (`E_APPROVAL_PENDING`); `proposal` is a change a run offers to
// a document (`ProposalCard`, client/Spec.lean §4-55).
export type DecideKind = "question" | "ask" | "proposal";

// The three answers, each on its key.
export type Answer = "yes" | "edit" | "no";

export interface Choice {
  readonly answer: Answer;
  readonly label: string;
  // Why the answer cannot be given now; the control stays reachable
  // and says so (client/Spec.lean §7-2).
  readonly why?: string | undefined;
  readonly onPress: () => void;
}

export interface DecideProps {
  readonly kind: DecideKind;
  // Who asks, in words; the heading of the card and its name.
  readonly asker: string;
  // When it was asked, already in the reader's clock. A proposal card
  // carries no moment on the wire, so its card states the version it
  // was made on here instead: the one thing that places it.
  readonly at: string;
  // The kind's own body: what is asked, and what it is about.
  readonly body: Snippet;
  // At most one choice per answer, in the order yes, edit, no.
  readonly choices: readonly Choice[];
}

// **The glyph is the encoding, the colour only repeats it**: a
// forced-colour mode repaints the bar and leaves the drawing, so each
// kind is its own drawing. A kind is added by a row here and a body at
// the call site; the props do not change.
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

// The choices the card offers: at most one per answer, in the order
// yes, edit, no, whatever order the caller handed them in.
export function ordered(choices: readonly Choice[]): readonly Choice[] {
  return ORDER.flatMap((answer) => choices.filter((choice) => choice.answer === answer).slice(0, 1));
}

// The choice a chord answers: the one the card offers on that chord's
// action, and only while it can be given. `Decide.lean` property 4.
export function answering(offered: readonly Choice[], action: Action | null): Choice | undefined {
  const chosen = offered.find((choice) => KEY[choice.answer] === action);
  return chosen === undefined || chosen.why !== undefined ? undefined : chosen;
}

// One answer as the look draws it: the button, and the chord beside it.
export interface AnswerLook {
  readonly answer: Answer;
  readonly label: string;
  readonly tone: "primary" | "quiet";
  readonly why: string | undefined;
  readonly onPress: () => void;
  readonly marks: readonly string[];
}

export interface DecideWiring {
  // Spread on the card: one region named by its heading, one entry of
  // the list it stands in, and the listener its chords are heard by.
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
  // The chord each action is on, as this machine writes it.
  readonly marksOf: (action: Action) => readonly string[];
}

const HEAR = createAttachmentKey();

export function lookOf(props: Omit<DecideProps, "body">, deciding: Deciding): DecideWiring {
  const asker = { id: `${deciding.uid}-asker`, text: props.asker };
  return {
    // Its own focus is where `j`/`k` and the digits of the mailbox land
    // (client/Spec.lean §7-11), and where its three chords are heard.
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
