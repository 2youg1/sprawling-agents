// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Doing } from "../../core/doing";
import { POSTURE_WORD } from "../../core/doing";
import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { buildingOf, roomOf, toFragment } from "../../core/route";
import { count, isoInstant, lasted, moment, usd } from "../../core/time";
import type { Address, Closing, Turn } from "../../wire";
import { figuresOf, modelsOf } from "./lanes";

export interface HeadProps {
  readonly turns: readonly Turn[];
  readonly doing: Doing | undefined;
  readonly closing: Closing | null;
  readonly room: Address | null;

  readonly dispatchedBy: string | null;

  readonly from: number | null;
  readonly to: number | null;
}

export type Rank = "value" | "more";

export type Face = "words" | "figure" | "mono";

export interface LinkWire {
  readonly href: string;
}

export interface LinkLook {
  readonly text: string;
  readonly wire: LinkWire;
}

export type LineLook =
  | {
      readonly kind: "text";
      readonly key: string;
      readonly rank: Rank;
      readonly face: Face;
      readonly text: string;
    }
  | { readonly kind: "time"; readonly key: string; readonly text: string; readonly at: string }
  | {
      readonly kind: "place";
      readonly key: string;
      readonly building: LinkLook;
      readonly room: LinkLook;
    };

export interface CellLook {
  readonly key: string;
  readonly name: string;

  readonly wide: boolean;
  readonly lines: readonly LineLook[];
}

export interface HeadLook {
  readonly label: string;
  readonly cells: readonly CellLook[];
}

export function lookOf(props: HeadProps, lang: Lang): HeadLook {
  const cells: (CellLook | null)[] = [
    outcome(props, lang),
    took(props, lang),
    tokens(props.turns, lang),
    spent(props.turns, lang),
    models(props.turns, lang),
    origin(props.room, lang),
    dispatched(props.dispatchedBy, lang),
  ];
  return { label: say(lang, "run_head_region"), cells: cells.filter((cell) => cell !== null) };
}

function text(key: string, rank: Rank, face: Face, words: string): LineLook {
  return { kind: "text", key, rank, face, text: words };
}

function outcome(props: HeadProps, lang: Lang): CellLook {
  const completion = props.closing?.completion ?? "";
  return {
    key: "outcome",
    name: say(lang, "run_head_outcome"),
    wide: false,
    lines: [
      text("posture", "value", "words", say(lang, POSTURE_WORD[props.doing?.kind ?? "unknown"])),
      ...(completion === "" ? [] : [text("completion", "more", "words", completion)]),
    ],
  };
}

function took(props: HeadProps, lang: Lang): CellLook | null {
  const { from, to } = props;
  if (from === null || to === null) return null;
  return {
    key: "took",
    name: say(lang, "run_head_took"),
    wide: true,
    lines: [
      text("lasted", "value", "figure", lasted(to - from)),
      {
        kind: "time",
        key: "started",
        text: fill(say(lang, "run_head_started"), { clock: moment(from) }),
        at: isoInstant(from),
      },
    ],
  };
}

function tokens(turns: readonly Turn[], lang: Lang): CellLook {
  const figures = figuresOf(turns);
  return {
    key: "tokens",
    name: say(lang, "run_head_tokens"),
    wide: false,
    lines: [
      text("in-out", "value", "figure", `${count(figures.input)} / ${count(figures.output)}`),
      text(
        "cached",
        "more",
        "figure",
        figures.cached === null
          ? say(lang, "run_head_cached_unknown")
          : fill(say(lang, "run_head_cached"), { n: count(figures.cached) }),
      ),
    ],
  };
}

function spent(turns: readonly Turn[], lang: Lang): CellLook {
  return {
    key: "spent",
    name: say(lang, "run_head_spent"),
    wide: false,
    lines: [
      text("usd", "value", "figure", usd(figuresOf(turns).usd)),
      text(
        "turns",
        "more",
        "figure",
        fill(say(lang, "run_head_turns"), { n: String(turns.length) }),
      ),
    ],
  };
}

function models(turns: readonly Turn[], lang: Lang): CellLook | null {
  const named = modelsOf(turns);
  if (named.length === 0) return null;
  return {
    key: "model",
    name: say(lang, "run_head_model"),
    wide: false,
    lines: named.map((model) => text(model, "value", "mono", model)),
  };
}

function origin(room: Address | null, lang: Lang): CellLook | null {
  if (room === null) return null;
  const building = buildingOf(room);
  return {
    key: "origin",
    name: say(lang, "run_head_origin"),
    wide: false,
    lines: [
      {
        kind: "place",
        key: room,
        building: {
          text: building,
          wire: { href: toFragment({ kind: "building", address: building }) },
        },
        room: { text: roomOf(room), wire: { href: toFragment({ kind: "talk", address: room }) } },
      },
    ],
  };
}

function dispatched(by: string | null, lang: Lang): CellLook | null {
  if (by === null) return null;
  return {
    key: "dispatched",
    name: say(lang, "run_head_dispatched_by"),
    wide: false,
    lines: [text("by", "value", "mono", by)],
  };
}
