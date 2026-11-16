// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { HunksAnswer } from "../../wire";
import { numbered } from "../changes";
import type { CodeLine } from "../changes";

const SIGN: Readonly<Record<CodeLine["kind"], string>> = {
  added: "+",
  removed: "\u2212",
  context: "",
};

export interface PatchWords {
  readonly withheld: (n: string, reason: string) => string;
  readonly folded: (n: string) => string;
  readonly quote: (n: string) => string;
}

export interface PatchHands {
  readonly href: string | undefined;
  readonly choose: (line: CodeLine) => void;
}

export interface QuoteWire {
  readonly href: string;
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export type PatchRow =
  | { readonly kind: "withheld"; readonly key: number; readonly text: string }
  | { readonly kind: "head"; readonly key: number; readonly text: string }
  | {
      readonly kind: "hunk";
      readonly key: number;
      readonly folded: string | undefined;
      readonly text: string;
    }
  | {
      readonly kind: "code";
      readonly key: number;
      readonly tone: CodeLine["kind"];
      readonly place: string;
      readonly sign: string;
      readonly text: string;
      readonly chosen: boolean;
      readonly quote: QuoteWire | undefined;
    };

export interface PatchLook {
  readonly source: string;
  readonly rows: readonly PatchRow[];
}

export function lookOf(
  patch: HunksAnswer,
  cursor: number | null,
  words: PatchWords,
  hands: PatchHands,
): PatchLook {
  return {
    source: patch.path,
    rows: numbered(patch).map((line): PatchRow => {
      switch (line.kind) {
        case "withheld":
          return {
            kind: "withheld",
            key: line.number,
            text: words.withheld(String(line.number), line.reason),
          };
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

function codeRow(
  line: CodeLine,
  cursor: number | null,
  words: PatchWords,
  hands: PatchHands,
): PatchRow {
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
