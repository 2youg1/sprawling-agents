// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the first screen knows about colouring code: the names of the
// inks, and where to fetch the highlighter that assigns them. The
// highlighter and its grammars are a lazy chunk (`paint.ts`), fetched
// the first time a file, a code block or the monitor shows code, so a
// screen that shows none downloads none of it (client/Spec.lean §4-26).

// What a stretch of code is, as far as the grammar says. Four classes
// and a fifth for everything else, because the theme offers four inks
// a reader can hold apart at note size.
export type Ink = "plain" | "comment" | "string" | "number" | "word";

export interface Piece {
  readonly ink: Ink;
  readonly text: string;
}

// The whole text as one run of pieces, newlines included, so the column
// that draws it is a flat list rather than a box per line. `source` is a
// file path or a Markdown fence word; either picks the grammar, and one
// no grammar answers to is one run of plain ink. A chunk that cannot be
// fetched - the server went away between two screens - is the same
// plain run, because the text is still worth reading uncoloured.
export function painted(text: string, source: string): Promise<readonly Piece[]> {
  return import("./paint").then(
    (highlighter) => highlighter.paint(text, source),
    (): readonly Piece[] => [{ ink: "plain", text }],
  );
}
