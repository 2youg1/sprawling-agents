// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One settings card (client/Spec.lean §4-36): what its look is given,
// and the one reading of what its foot says about a save. The foot is
// the same sentence on every card, the ones saved by a press and the
// ones saved by a pick, so a card says "saved" on the same evidence
// wherever it stands (`saving.ts`).

import type { Snippet } from "svelte";

import type { Key } from "../../core/lang";
import type { Saving } from "./saving";

// What the foot says about the save. `word` is where the save stands,
// `detail` the sentence under it: when a saved change takes effect, or
// the city's own way on after a refusal. A card with no button says
// when a change takes effect before anything is saved, because that
// sentence is the only constraint a pick has.
export interface Standing {
  readonly word: string | null;
  readonly weight: "quiet" | "alert";
  readonly saved: boolean;
  readonly detail: string | null;
}

// The save button of a card saved by a press.
export interface Press {
  readonly label: string;
  readonly loading: boolean;
  // Present means there is nothing to save, and says so.
  readonly why: string | undefined;
  readonly onPress: () => void;
}

// Everything a card's look is given; every word already in the
// person's language.
export interface CardLook {
  readonly title: string;
  readonly note: string;
  readonly standing: Standing;
  readonly press: Press | null;
  readonly children: Snippet;
}

const WORD: Record<Saving["kind"], Key | null> = {
  held: null,
  draft: "saving_draft",
  saving: "saving_sent",
  saved: "saving_saved",
  refused: "saving_refused",
  unverified: "saving_unverified",
};

// What the foot says, from where the save stands, the sentence that
// says when a change takes effect, and whether the card has a button.
export function standingOf(saving: Saving, settled: string, pressed: boolean, words: (key: Key) => string): Standing {
  const word = WORD[saving.kind];
  return {
    word: word === null ? null : words(word),
    weight: saving.kind === "refused" ? "alert" : "quiet",
    saved: saving.kind === "saved",
    detail: detailOf(saving, settled, pressed),
  };
}

function detailOf(saving: Saving, settled: string, pressed: boolean): string | null {
  switch (saving.kind) {
    case "saved":
      return settled;
    case "refused":
      return saving.error.recovery;
    case "held":
    case "draft":
    case "saving":
    case "unverified":
      return pressed ? null : settled;
  }
}

// Whether a press would send anything: a draft, a refused draft, or a
// save the city never answered.
export function pressable(saving: Saving): boolean {
  switch (saving.kind) {
    case "draft":
    case "refused":
    case "unverified":
      return true;
    case "held":
    case "saving":
    case "saved":
      return false;
  }
}
