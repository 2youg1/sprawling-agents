// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a proposal card's body draws, and why each answer cannot be
// given now (client/Spec.lean §4-55). The seat (`proposals_card.svelte`)
// holds the card's state and its effects; this file turns that state
// into `CardLook`, the whole value `proposals_card.look.svelte` draws:
// every word already in the person's language, and every box a person
// operates as a wire bag the look spreads on it unchanged. A look that
// draws the card some other way takes the same value, so the edit's
// checkboxes and text boxes keep their names and their hands.

import type { Snippet } from "svelte";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { ProposalCard, Slice } from "../../wire";
import { takesAny } from "./proposals";
import type { Deciding, Edit, Standing, Take } from "./proposals";
import { short } from "./reading";

// The bag spread on a changed sentence's take box: whether the edit
// takes the sentence.
export interface TakeWire {
  readonly type: "checkbox";
  readonly checked: boolean;
  readonly "aria-label": string;
  readonly onchange: (event: { readonly currentTarget: { readonly checked: boolean } }) => void;
}

// The bag spread on an inserted sentence's text box: the words it lands
// as.
export interface WordsWire {
  readonly rows: 1;
  readonly "aria-label": string;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

// One sentence of the card under edit, keyed by its place on the card.
// A sentence that does not change has no box; a removed one can be
// left out; an inserted one can be left out or rewritten.
export type RowLook =
  | { readonly key: number; readonly kind: "kept"; readonly text: string }
  | { readonly key: number; readonly kind: "struck"; readonly text: string; readonly taken: boolean; readonly take: TakeWire }
  | { readonly key: number; readonly kind: "written"; readonly taken: boolean; readonly take: TakeWire; readonly words: WordsWire };

// One sentence of the diff as the city will land it, with its own
// spacing. `said` is the word a screen reader hears before a changed
// sentence; `abuts` sets an inserted sentence half a character apart
// from a struck one it would otherwise touch.
export interface PieceLook {
  readonly key: number;
  readonly kind: Slice["kind"];
  readonly lead: string;
  readonly text: string;
  readonly trail: string;
  readonly said: string | undefined;
  readonly abuts: boolean;
}

export type BodyLook =
  | { readonly kind: "diff"; readonly pieces: readonly PieceLook[] }
  | { readonly kind: "edit"; readonly rows: readonly RowLook[] };

// Where a sent decision stands, in words.
export interface StatusLook {
  readonly tone: "faint" | "alert";
  readonly text: string;
}

// Everything the card's look is given.
export interface CardLook {
  // The seat's own line above the diff, when it hands one in.
  readonly lead: Snippet | undefined;
  // Which version the card was made on and which the document holds,
  // when the two differ.
  readonly stale: string | undefined;
  // The live region a sent decision speaks in. It is drawn before it
  // has anything to say, so the first word it says is announced.
  readonly region: { readonly role: "status" };
  readonly status: StatusLook | undefined;
  readonly body: BodyLook;
}

// The card's state as the seat holds it. `edit` is the person's edit
// while the card is open for editing, and `null` while it shows the
// diff.
export interface Held {
  readonly standing: Standing;
  readonly deciding: Deciding;
  readonly edit: Edit | null;
  readonly lead: Snippet | undefined;
}

// What only the seat can do: change one take of the edit it holds.
export interface Hands {
  readonly retake: (place: number, change: Partial<Take>) => void;
}

export function cardLookOf(card: ProposalCard, held: Held, lang: Lang, hands: Hands): CardLook {
  return {
    lead: held.lead,
    stale: staleLine(card, held.standing, lang),
    region: { role: "status" },
    status: statusOf(held.deciding, lang),
    body:
      held.edit === null
        ? { kind: "diff", pieces: card.slices.map((slice, place) => pieceOf(card, slice, place, lang)) }
        : { kind: "edit", rows: rowsOf(card, held.edit, lang, hands) },
  };
}

// Why each answer cannot be given now, or `undefined` for one that can.
// A card in flight answers nothing; a stale card can only be rejected;
// an edit that takes nothing is a rejection, so it is not sent as an
// acceptance.
export interface Refusals {
  readonly accept: string | undefined;
  readonly edit: string | undefined;
  readonly reject: string | undefined;
}

export function refusalsOf(held: Omit<Held, "lead">, lang: Lang): Refusals {
  const busy = held.deciding.kind === "sent" || held.deciding.kind === "pending";
  const stale = held.standing.kind === "stale";
  const busyWhy = busy ? say(lang, "proposal_busy_why") : undefined;
  const staleWhy = stale ? say(lang, "proposal_stale_why") : undefined;
  const emptyWhy = held.edit !== null && !takesAny(held.edit) ? say(lang, "proposal_nothing_taken") : undefined;
  return {
    accept: busyWhy ?? staleWhy ?? emptyWhy,
    edit: busyWhy ?? staleWhy,
    reject: busyWhy,
  };
}

function staleLine(card: ProposalCard, standing: Standing, lang: Lang): string | undefined {
  if (standing.kind === "current") return undefined;
  return fill(say(lang, "proposal_stale"), {
    was: short(card.baseline),
    now: standing.now === null ? say(lang, "proposal_no_version") : short(standing.now),
  });
}

function statusOf(deciding: Deciding, lang: Lang): StatusLook | undefined {
  switch (deciding.kind) {
    case "idle":
      return undefined;
    case "sent":
      return { tone: "faint", text: say(lang, "proposal_deciding") };
    case "pending":
      return { tone: "faint", text: say(lang, "proposal_pending") };
    case "refused":
      return { tone: "alert", text: deciding.error.recovery };
  }
}

function pieceOf(card: ProposalCard, slice: Slice, place: number, lang: Lang): PieceLook {
  const said = slice.kind === "same" ? undefined : say(lang, slice.kind === "delete" ? "proposal_removed" : "proposal_added");
  return {
    key: place,
    kind: slice.kind,
    lead: slice.lead,
    text: slice.text,
    trail: slice.trail,
    said,
    abuts: slice.kind === "insert" && abuts(card, place),
  };
}

function rowsOf(card: ProposalCard, edit: Edit, lang: Lang, hands: Hands): readonly RowLook[] {
  return card.slices.map((slice, place): RowLook => {
    const take = edit.get(place);
    if (slice.kind === "same" || take === undefined) return { key: place, kind: "kept", text: slice.text };
    const box: TakeWire = {
      type: "checkbox",
      checked: take.taken,
      "aria-label": fill(say(lang, "proposal_take"), { words: opening(slice) }),
      onchange: (event) => {
        hands.retake(place, { taken: event.currentTarget.checked });
      },
    };
    if (slice.kind === "delete") return { key: place, kind: "struck", text: slice.text, taken: take.taken, take: box };
    return {
      key: place,
      kind: "written",
      taken: take.taken,
      take: box,
      words: {
        rows: 1,
        "aria-label": say(lang, "proposal_amend"),
        value: take.text,
        oninput: (event) => {
          hands.retake(place, { text: event.currentTarget.value });
        },
      },
    };
  });
}

// Whether the sentence at `place` meets a struck one with no space
// between them; the diff then sets the two half a character apart, so
// the removed and the added words do not read as one.
function abuts(card: ProposalCard, place: number): boolean {
  const before = card.slices[place - 1];
  const here = card.slices[place];
  return before?.kind === "delete" && before.trail === "" && here?.lead === "";
}

// The first words of a sentence, for the name of its take box.
function opening(slice: Slice): string {
  const words = slice.text.trim().split(/\s+/u).slice(0, 6).join(" ");
  return words.length < slice.text.trim().length ? `${words}…` : words;
}
