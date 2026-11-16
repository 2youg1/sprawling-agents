// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Snippet } from "svelte";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { ProposalCard, Slice } from "../../wire";
import { takesAny } from "./proposals";
import type { Deciding, Edit, Standing, Take } from "./proposals";
import { short } from "./reading";


export interface TakeWire {
  readonly type: "checkbox";
  readonly checked: boolean;
  readonly "aria-label": string;
  readonly onchange: (event: { readonly currentTarget: { readonly checked: boolean } }) => void;
}


export interface WordsWire {
  readonly rows: 1;
  readonly "aria-label": string;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
}


export type RowLook =
  | { readonly key: number; readonly kind: "kept"; readonly text: string }
  | {
      readonly key: number;
      readonly kind: "struck";
      readonly text: string;
      readonly taken: boolean;
      readonly take: TakeWire;
    }
  | {
      readonly key: number;
      readonly kind: "written";
      readonly taken: boolean;
      readonly take: TakeWire;
      readonly words: WordsWire;
    };


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


export interface StatusLook {
  readonly tone: "faint" | "alert";
  readonly text: string;
}


export interface CardLook {
  
  readonly lead: Snippet | undefined;
  
  
  readonly stale: string | undefined;
  
  
  readonly region: { readonly role: "status" };
  readonly status: StatusLook | undefined;
  readonly body: BodyLook;
}


export interface Held {
  readonly standing: Standing;
  readonly deciding: Deciding;
  readonly edit: Edit | null;
  readonly lead: Snippet | undefined;
}


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
        ? {
            kind: "diff",
            pieces: card.slices.map((slice, place) => pieceOf(card, slice, place, lang)),
          }
        : { kind: "edit", rows: rowsOf(card, held.edit, lang, hands) },
  };
}


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
  const emptyWhy =
    held.edit !== null && !takesAny(held.edit) ? say(lang, "proposal_nothing_taken") : undefined;
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
  const said =
    slice.kind === "same"
      ? undefined
      : say(lang, slice.kind === "delete" ? "proposal_removed" : "proposal_added");
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
    if (slice.kind === "same" || take === undefined)
      return { key: place, kind: "kept", text: slice.text };
    const box: TakeWire = {
      type: "checkbox",
      checked: take.taken,
      "aria-label": fill(say(lang, "proposal_take"), { words: opening(slice) }),
      onchange: (event) => {
        hands.retake(place, { taken: event.currentTarget.checked });
      },
    };
    if (slice.kind === "delete")
      return { key: place, kind: "struck", text: slice.text, taken: take.taken, take: box };
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


function abuts(card: ProposalCard, place: number): boolean {
  const before = card.slices[place - 1];
  const here = card.slices[place];
  return before?.kind === "delete" && before.trail === "" && here?.lead === "";
}


function opening(slice: Slice): string {
  const words = slice.text.trim().split(/\s+/u).slice(0, 6).join(" ");
  return words.length < slice.text.trim().length ? `${words}…` : words;
}
