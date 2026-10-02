// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A model's reply as the page lays it out: the page's half of
// `Query::Reply` (client-SPEC 4-26, 12-31). The city reads the Markdown
// and says where the blocks it read end; this decides which stretch of
// the text the next question carries and how an answer joins the blocks
// already drawn. Nothing here reads Markdown.

import type { Answered } from "./answered";
import { utf16At } from "./document_pos";
import type { Block, Laid, ReplyState } from "../wire";

export interface Laying {
  // Every block drawn so far, in the order the answers came.
  readonly blocks: readonly Block[];
  // How much of the text the blocks cover, in UTF-16 code units; the
  // text after it is drawn as it is.
  readonly reached: number;
  // The stretch past `reached` the city has already read and left open,
  // which a question would only ask again.
  readonly judged: string | null;
  // The city answered `Unavailable`: the rest stays raw and is not asked.
  readonly refused: boolean;
}

export const UNLAID: Laying = { blocks: [], reached: 0, judged: null, refused: false };

// The text the next question carries, or `null` when no question is due.
// A streaming question carries only the complete lines of the rest,
// because a closure point falls after a complete line and nowhere else
// (`crates/documents/Spec.lean` D31): a token that brings no line break
// cannot move it.
export function question(text: string, laying: Laying, state: ReplyState): string | null {
  if (laying.refused) return null;
  const rest = text.slice(laying.reached);
  const asked = state === "streaming" ? rest.slice(0, rest.lastIndexOf("\n") + 1) : rest;
  return asked === "" || asked === laying.judged ? null : asked;
}

// The laying once the city answered the question that carried `asked`.
//
// A streaming answer stops at the closure point, so the lines it was
// sent past that point are judged open until another line arrives. A
// settled answer stops short of the text's end only when it filled a
// window, so the rest is asked next - unless it read nothing at all.
export function answered(laying: Laying, asked: string, read: Answered<Laid>, state: ReplyState): Laying {
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
