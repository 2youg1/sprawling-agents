// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the first screen knows about colouring code: the names of the
// inks, and where to fetch the highlighter that assigns them. The
// highlighter and its grammars are a lazy chunk (`paint.ts`), fetched
// the first time a file, a code block or the monitor shows code, so a
// screen that shows none downloads none of it (client/Spec.lean §4-26).

import type { Snippet } from "svelte";
import type { Attachment } from "svelte/attachments";

import type { CopyProps } from "./copy";

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

// What a look of the inked run is given: the pieces, plain until the
// highlighter answers for this very text (`inked.svelte`).
export interface InkedLook {
  readonly pieces: readonly Piece[];
}

// One segment of the file's trail. Each carries its own identity, so a
// path whose segment repeats - `src/.../src/...` - keys no two alike.
export interface Crumb {
  readonly part: string;
  readonly last: boolean;
}

// What a look of the code view is given (`code.svelte`). The two
// attachments go on the scrolling box and on the cited line's mark, so
// the seat can bring the mark to the middle of the view.
export interface CodeLook {
  readonly crumbs: readonly Crumb[];
  // Spread on the trail's landmark: its name, in the reader's language.
  readonly trail: { readonly "aria-label": string };
  // The line numbers, one per line of `text`, joined by newlines.
  readonly gutter: string;
  readonly text: string;
  readonly path: string;
  // The line a reader was sent to, one-based, or `undefined`.
  readonly cited: number | undefined;
  readonly copy: CopyProps;
  readonly scroller: Attachment<HTMLElement>;
  readonly mark: Attachment<HTMLElement>;
  readonly aside: Snippet | undefined;
}

export function crumbsOf(path: string): readonly Crumb[] {
  const parts = path.split("/").filter((part) => part !== "");
  return parts.map((part, at) => ({ part, last: at === parts.length - 1 }));
}

// Line numbers start at one because the wire hands over the head of a
// result and says nothing about where in the file it began. The day a
// call carries that offset, this takes it rather than deciding it here
// (client/Spec.lean §4-26).
export function gutterOf(text: string): string {
  return text
    .split("\n")
    .map((_line, at) => String(at + 1))
    .join("\n");
}
