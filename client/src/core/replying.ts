// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Answered } from "./answered";
import { utf16At } from "./document_pos";
import type { Block, Laid, ReplyState } from "../wire";

export interface Laying {

  readonly blocks: readonly Block[];

  readonly reached: number;

  readonly judged: string | null;

  readonly refused: boolean;
}

export const UNLAID: Laying = { blocks: [], reached: 0, judged: null, refused: false };

export function question(text: string, laying: Laying, state: ReplyState): string | null {
  if (laying.refused) return null;
  const rest = text.slice(laying.reached);
  const asked = state === "streaming" ? rest.slice(0, rest.lastIndexOf("\n") + 1) : rest;
  return asked === "" || asked === laying.judged ? null : asked;
}

export function answered(
  laying: Laying,
  asked: string,
  read: Answered<Laid>,
  state: ReplyState,
): Laying {
  switch (read.kind) {
    case "asking":
      return laying;
    case "unavailable":
      return { ...laying, refused: true };
    case "held": {
      const end = utf16At(asked, read.value.span.end);
      return {
        blocks: [...laying.blocks, ...read.value.blocks],
        reached: laying.reached + end,
        judged: state === "streaming" ? asked.slice(end) : end === 0 ? asked : null,
        refused: false,
      };
    }
  }
}
