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
export function question(text: string, laying: Laying, state: ReplyState): string | null {
  return text === "" || laying.refused || state === "settled" ? null : null;
}

// The laying once the city answered the question that carried `asked`.
export function answered(laying: Laying, asked: string, read: Answered<Laid>, state: ReplyState): Laying {
  return asked === "" || read.kind === "asking" || state === "settled" ? laying : laying;
}
