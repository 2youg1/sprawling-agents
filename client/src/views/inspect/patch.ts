// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one file's patch shows, apart from how it is drawn (client D95):
// the rows `changes.ts`'s `numbered` walks out of the patch, each code
// row with the one number it has in the tree it lives in, and - where
// the page has a conversation - the wire bag of the number that quotes
// the line into it. Nothing here touches the DOM.

import type { HunksAnswer } from "../../wire";
import { numbered } from "../changes";
import type { CodeLine } from "../changes";

// The patch's own marks for the three kinds of code line.
const SIGN: Readonly<Record<CodeLine["kind"], string>> = { added: "+", removed: "\u2212", context: "" };

// The patch's words, already in the person's language.
export interface PatchWords {
  readonly withheld: (n: string, reason: string) => string;
  readonly folded: (n: string) => string;
  readonly quote: (n: string) => string;
}

// What only the seat holds: the link to the conversation a chosen line
// is quoted into, absent where the page has none, and what choosing a
// line does there.
export interface PatchHands {
  readonly href: string | undefined;
  readonly choose: (line: CodeLine) => void;
}

// The bag spread on a code row's number where the line can be quoted.
export interface QuoteWire {
  readonly href: string;
  readonly "aria-label": string;
  readonly onclick: () => void;
}

// One drawn row, keyed by its place in the patch text.
export type PatchRow =
  | { readonly kind: "withheld"; readonly key: number; readonly text: string }
  | { readonly kind: "head"; readonly key: number; readonly text: string }
  | { readonly kind: "hunk"; readonly key: number; readonly folded: string | undefined; readonly text: string }
  | {
      readonly kind: "code";
      readonly key: number;
      readonly tone: CodeLine["kind"];
      // The line's number in its own tree, empty where it is unknown.
      readonly place: string;
      readonly sign: string;
      readonly text: string;
      // The line a person last chose.
      readonly chosen: boolean;
      readonly quote: QuoteWire | undefined;
    };

// Everything a look is given.
export interface PatchLook {
  // The file's path, which picks the grammar the lines are inked in.
  readonly source: string;
  readonly rows: readonly PatchRow[];
}

// The whole value a look draws.
export function lookOf(patch: HunksAnswer, cursor: number | null, words: PatchWords, hands: PatchHands): PatchLook {
  return {
    source: patch.path,
    rows: numbered(patch).map((line): PatchRow => {
      switch (line.kind) {
        case "withheld":
          return { kind: "withheld", key: line.number, text: words.withheld(String(line.number), line.reason) };
        case "head":
          return { kind: "head", key: line.number, text: line.text };
        case "hunk":
          return {
            kind: "hunk",
            key: line.number,
            folded: line.folded > 0 ? words.folded(String(line.folded)) : undefined,
            text: line.text,
          };
        case "added":
        case "removed":
        case "context":
          return codeRow(line, cursor, words, hands);
      }
    }),
  };
}

function codeRow(line: CodeLine, cursor: number | null, words: PatchWords, hands: PatchHands): PatchRow {
  const place = line.kind === "removed" ? line.old : line.new;
  const { href } = hands;
  return {
    kind: "code",
    key: line.number,
    tone: line.kind,
    place: place === null ? "" : String(place),
    sign: SIGN[line.kind],
    text: line.text,
    chosen: cursor === line.number,
    quote:
      href === undefined
        ? undefined
        : {
            href,
            "aria-label": words.quote(String(place ?? line.number)),
            onclick: () => {
              hands.choose(line);
            },
          },
  };
}
