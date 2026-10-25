// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the prompt lens is given (client D95: a seat, a look and this
// file): the tool table the run's requests carried ahead of the system
// prompt, then the four segments the run was sent, each in the person's
// language and each with its presses decided, so a look draws the lens
// by taking `PromptLook` and nothing else.
//
// A segment folds on its own, states the files it was read from and
// what the budget cut off the end of them, and copies as the text the
// model saw. A segment the store no longer holds says so and offers no
// copy, rather than a key that would copy an empty prompt.

import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { count } from "../../core/time";
import type { Address, Exposure, PrefixAnswer, PrefixSegment, PrefixSlot, PromptTools } from "../../wire";

// Each slot's word, which the building page's prompts section reads too.
export const SLOT_WORD: Record<PrefixSlot, Key> = {
  city: "slot_city",
  building: "slot_building",
  resident: "slot_resident",
  run: "slot_run",
};

// Spread on the row's own button: it opens and closes the segment.
export interface FoldWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

// Spread on the copy key.
export interface CopyWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface CopyLook {
  // The key's word: what it does before the press, the receipt after.
  readonly label: string;
  // Whether the receipt is showing, which the look marks with a check.
  readonly copied: boolean;
  readonly wire: CopyWire;
}

export interface SourceLook {
  readonly key: string;
  readonly path: string;
  readonly onOpen: () => void;
  // What the budget cut off this file, in words; absent when nothing was.
  readonly dropped: string | undefined;
}

export interface SegmentLook {
  readonly key: string;
  readonly slot: string;
  readonly size: string;
  // Present when the store no longer holds the bytes, and says so.
  readonly gone: string | undefined;
  readonly open: boolean;
  readonly fold: FoldWire;
  readonly copy: CopyLook | undefined;
  readonly sourcesLabel: string;
  readonly sources: readonly SourceLook[];
  readonly text: string;
}

// The tool table, which folds like a segment and lists one tool a line.
export interface ToolsLook {
  readonly key: string;
  readonly slot: string;
  readonly size: string;
  readonly open: boolean;
  readonly fold: FoldWire;
  readonly rows: readonly { readonly key: string; readonly name: string; readonly facts: string }[];
}

export interface PromptLook {
  // Absent for a run frozen before its tool table was recorded.
  readonly tools: ToolsLook | undefined;
  // `undefined` while the city has not answered.
  readonly segments: readonly SegmentLook[] | undefined;
  // What the lens says when the run was sent no prompt at all.
  readonly none: string;
}

// What the seat holds: which segments are open and whose receipt shows.
export interface Held {
  readonly open: ReadonlySet<string>;
  readonly receipt: string | null;
}

// The presses the seat carries out.
export interface Hands {
  readonly toggle: (hash: string) => void;
  readonly copy: (segment: PrefixSegment) => void;
  readonly visit: (source: Address) => void;
}

// What the run was sent: the answer's segments and tool table.
export type Sent = Pick<PrefixAnswer, "segments" | "tools">;

export function lookOf(sent: Sent | undefined, held: Held, lang: Lang, hands: Hands): PromptLook {
  const tools = sent?.tools ?? null;
  return {
    tools: tools === null ? undefined : toolsOf(tools, held, lang, hands),
    segments: sent?.segments.map((segment) => segmentOf(segment, held, lang, hands)),
    none: say(lang, "run_no_prompt"),
  };
}

// The key the tool table folds under, which no segment hash can be.
const TOOLS_KEY = "tools";

const EXPOSURE_WORD: Record<Exposure, Key> = {
  core: "catalog_core",
  dormant: "catalog_dormant",
  off: "catalog_off",
};

function toolsOf(tools: PromptTools, held: Held, lang: Lang, hands: Hands): ToolsLook {
  const open = held.open.has(TOOLS_KEY);
  return {
    key: TOOLS_KEY,
    slot: say(lang, "run_prompt_tools"),
    size: fill(say(lang, "run_prompt_tools_size"), {
      n: count(tools.len),
      core: String(tools.rows.filter((row) => row.exposure === "core").length),
      all: String(tools.rows.length),
    }),
    open,
    fold: {
      type: "button",
      "aria-expanded": open,
      onclick: () => {
        hands.toggle(TOOLS_KEY);
      },
    },
    rows: tools.rows.map((row) => ({
      key: row.name,
      name: row.name,
      facts: [
        row.namespace,
        say(lang, EXPOSURE_WORD[row.exposure]),
        fill(say(lang, "run_prompt_bytes"), { n: count(row.len) }),
      ].join(" · "),
    })),
  };
}

function segmentOf(segment: PrefixSegment, held: Held, lang: Lang, hands: Hands): SegmentLook {
  const open = held.open.has(segment.hash);
  return {
    key: segment.hash,
    slot: say(lang, SLOT_WORD[segment.slot]),
    size: fill(say(lang, "run_prompt_bytes"), { n: count(segment.bytes) }),
    gone: segment.stored ? undefined : say(lang, "run_prompt_gone"),
    open,
    fold: {
      type: "button",
      "aria-expanded": open,
      onclick: () => {
        hands.toggle(segment.hash);
      },
    },
    copy: segment.stored ? copyOf(segment, held.receipt === segment.hash, lang, hands) : undefined,
    sourcesLabel: say(lang, "run_prompt_sources"),
    sources: segment.sources.map((source) => ({
      key: source.addr,
      path: source.addr,
      onOpen: () => {
        hands.visit(source.addr);
      },
      dropped: source.dropped > 0 ? fill(say(lang, "run_prompt_dropped"), { n: count(source.dropped) }) : undefined,
    })),
    text: segment.text,
  };
}

// The receipt replaces the word for its moment: the check is
// what a hand reads after the press, and the word is what it read
// before.
function copyOf(segment: PrefixSegment, copied: boolean, lang: Lang, hands: Hands): CopyLook {
  return {
    label: say(lang, copied ? "run_prompt_copied" : "run_prompt_copy"),
    copied,
    wire: {
      type: "button",
      onclick: () => {
        hands.copy(segment);
      },
    },
  };
}
