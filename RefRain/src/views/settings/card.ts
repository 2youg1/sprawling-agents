// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Snippet } from "svelte";

import type { Key } from "../../core/lang";
import type { Saving } from "./saving";


export interface Standing {
  readonly word: string | null;
  readonly weight: "quiet" | "alert";
  readonly saved: boolean;
  readonly detail: string | null;
}


export interface Press {
  readonly label: string;
  readonly loading: boolean;
  
  readonly why: string | undefined;
  readonly onPress: () => void;
}


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


export function standingOf(
  saving: Saving,
  settled: string,
  pressed: boolean,
  words: (key: Key) => string,
): Standing {
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
