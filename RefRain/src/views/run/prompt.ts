// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key, Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { count } from "../../core/time";
import type {
  Address,
  Exposure,
  PrefixAnswer,
  PrefixSegment,
  PrefixSlot,
  PromptTools,
} from "../../wire";

export const SLOT_WORD: Record<PrefixSlot, Key> = {
  city: "slot_city",
  building: "slot_building",
  resident: "slot_resident",
  run: "slot_run",
};

export interface FoldWire {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly onclick: () => void;
}

export interface CopyWire {
  readonly type: "button";
  readonly onclick: () => void;
}

export interface CopyLook {
  readonly label: string;
  readonly copied: boolean;
  readonly wire: CopyWire;
}

export interface SourceLook {
  readonly key: string;
  readonly path: string;
  readonly onOpen: () => void;
  readonly dropped: string | undefined;
}

export interface SegmentLook {
  readonly key: string;
  readonly slot: string;
  readonly size: string;
  readonly gone: string | undefined;
  readonly open: boolean;
  readonly fold: FoldWire;
  readonly copy: CopyLook | undefined;
  readonly sourcesLabel: string;
  readonly sources: readonly SourceLook[];
  readonly text: string;
}

export interface ToolsLook {
  readonly key: string;
  readonly slot: string;
  readonly size: string;
  readonly open: boolean;
  readonly fold: FoldWire;
  readonly rows: readonly { readonly key: string; readonly name: string; readonly facts: string }[];
}

export interface PromptLook {
  readonly tools: ToolsLook | undefined;
  readonly segments: readonly SegmentLook[] | undefined;
  readonly none: string;
}

export interface Held {
  readonly open: ReadonlySet<string>;
  readonly receipt: string | null;
}

export interface Hands {
  readonly toggle: (hash: string) => void;
  readonly copy: (segment: PrefixSegment) => void;
  readonly visit: (source: Address) => void;
}

export type Sent = Pick<PrefixAnswer, "segments" | "tools">;

export function lookOf(sent: Sent | undefined, held: Held, lang: Lang, hands: Hands): PromptLook {
  const tools = sent?.tools ?? null;
  return {
    tools: tools === null ? undefined : toolsOf(tools, held, lang, hands),
    segments: sent?.segments.map((segment) => segmentOf(segment, held, lang, hands)),
    none: say(lang, "run_no_prompt"),
  };
}

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
      dropped:
        source.dropped > 0
          ? fill(say(lang, "run_prompt_dropped"), { n: count(source.dropped) })
          : undefined,
    })),
    text: segment.text,
  };
}

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
