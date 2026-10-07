// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the runs board's look is handed, with no DOM and no runes: the
// heading and counts, the runs waiting for the person as one control
// each, the legend and the folded clock's scale, and the tree's rows
// that are drawn, each with its words said, its bar placed, and the
// wire bag that makes it a `treeitem` (client D95). The keys, the
// cursor, folding and the drawn window stay with the seat,
// `board.svelte`, which owns the state they change.

import type { Attachment } from "svelte/attachments";

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { FOLDS, along } from "./fold";
import type { BoardRun, Phase, Row } from "./lineage";
import { phaseOf } from "./lineage";
import { PHASES, PHASE_WORD, phaseSaid } from "./phase";

// What the seat listens to on the scrolling list: its keys, its scroll,
// and the element itself, held through a Svelte attachment
// (`createAttachmentKey`).
export interface TreeHands {
  readonly onkeydown: (event: KeyboardEvent) => void;
  readonly onscroll: (event: Event & { readonly currentTarget: EventTarget & HTMLElement }) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// Spread on the scrolling list that is the tree. The list is the one
// tab stop, and the cursor is its active descendant.
export interface TreeWire extends TreeHands {
  readonly role: "tree";
  readonly tabindex: 0;
  readonly "aria-label": string;
  readonly "aria-activedescendant": string | undefined;
}

// Spread on one row of the tree. A pointer opens the row it presses;
// the keyboard reaches rows through the tree's keys, not their own.
export interface RowWire {
  readonly id: string;
  readonly role: "treeitem";
  readonly "aria-level": number;
  readonly "aria-expanded": boolean | undefined;
  readonly "aria-selected": boolean;
  readonly onclick: () => void;
}

export type RowBody =
  | { readonly kind: "building"; readonly name: string; readonly asking: number; readonly runs: number }
  | { readonly kind: "room"; readonly name: string }
  | {
      readonly kind: "run";
      readonly phase: Phase;
      readonly id: string;
      readonly title: string;
      readonly said: string;
      // The run's life along the folded clock, in per cent of the bar.
      readonly bar: { readonly left: number; readonly width: number };
    };

export interface RowLook {
  readonly key: string;
  readonly current: boolean;
  readonly guide: string;
  readonly body: RowBody;
  readonly wire: RowWire;
}

export interface BoardLook {
  readonly level: 1 | 2;
  readonly title: string;
  readonly counts: string;
  readonly asking: {
    readonly label: string;
    readonly runs: readonly {
      readonly key: string;
      readonly text: string;
      readonly wire: { readonly type: "button"; readonly onclick: () => void };
    }[];
  } | null;
  readonly legend: { readonly label: string; readonly phases: readonly { readonly phase: Phase; readonly word: string }[] };
  // `wide` marks a fold the scale names only where the board has room.
  readonly folds: readonly { readonly minutes: number; readonly at: number; readonly label: string; readonly wide: boolean }[];
  readonly now: string;
  readonly tree: TreeWire;
  // The height, in pixels, of the rows above and below the drawn window,
  // which the list pads so its scroll height is the whole tree's.
  readonly pad: { readonly above: number; readonly below: number };
  readonly rows: readonly RowLook[];
  readonly keys: string;
}

// What the seat holds that this module reads: the runs, the clock, the
// language, the tree's rows with the cursor among them, the window of
// rows drawn and the height of one.
export interface BoardSeat {
  readonly runs: readonly BoardRun[];
  readonly now: number;
  readonly lang: Lang;
  readonly level: 1 | 2;
  readonly rows: readonly Row[];
  readonly folded: ReadonlySet<string>;
  readonly cursor: number;
  readonly shown: { readonly from: number; readonly to: number };
  readonly rowPx: number;
}

// What the seat does when the look's controls are used.
export interface BoardHands {
  readonly tree: TreeHands;
  readonly open: (row: Row) => void;
  readonly pick: (run: BoardRun) => void;
}

export function lookOf(seat: BoardSeat, hands: BoardHands): BoardLook {
  const { lang, now, rows, cursor } = seat;
  const waiting = seat.runs.filter((run) => phaseOf(run.doing) === "person");
  const held = rows[cursor];
  return {
    level: seat.level,
    title: say(lang, "runs_title"),
    counts: fill(say(lang, "runs_counts"), { asking: String(waiting.length), runs: String(seat.runs.length) }),
    asking:
      waiting.length === 0
        ? null
        : {
            label: say(lang, "runs_asking"),
            runs: waiting.map((run) => ({
              key: run.run,
              text: run.run.slice(0, 8),
              wire: {
                type: "button",
                onclick: () => {
                  hands.pick(run);
                },
              },
            })),
          },
    legend: { label: say(lang, "runs_legend"), phases: PHASES.map((phase) => ({ phase, word: say(lang, PHASE_WORD[phase]) })) },
    folds: FOLDS.filter((fold) => fold.minutes > 0).map((fold) => ({
      minutes: fold.minutes,
      at: fold.at,
      label: fill(say(lang, "runs_minus"), { n: String(fold.minutes) }),
      wide: fold.at !== 0,
    })),
    now: say(lang, "runs_now"),
    tree: {
      ...hands.tree,
      role: "tree",
      tabindex: 0,
      "aria-label": say(lang, "runs_tree"),
      "aria-activedescendant": held === undefined ? undefined : rowId(held),
    },
    pad: { above: seat.shown.from * seat.rowPx, below: (rows.length - seat.shown.to) * seat.rowPx },
    rows: rows.slice(seat.shown.from, seat.shown.to).map((row, at) => ({
      key: row.key,
      current: seat.shown.from + at === cursor,
      guide: row.guide,
      body: bodyOf(row, lang, now),
      wire: {
        id: rowId(row),
        role: "treeitem",
        "aria-level": levelOf(row),
        "aria-expanded": row.kind === "run" ? undefined : !seat.folded.has(row.key),
        "aria-selected": seat.shown.from + at === cursor,
        onclick: () => {
          hands.open(row);
        },
      },
    })),
    keys: say(lang, "runs_keys"),
  };
}

function rowId(row: Row): string {
  return `runs-row-${row.key}`;
}

function levelOf(row: Row): number {
  switch (row.kind) {
    case "building":
      return 1;
    case "room":
      return 2;
    case "run":
      return 3;
  }
}

function bodyOf(row: Row, lang: Lang, now: number): RowBody {
  switch (row.kind) {
    case "building":
      return { kind: "building", name: row.name, asking: row.asking, runs: row.runs };
    case "room":
      return { kind: "room", name: row.name };
    case "run":
      return {
        kind: "run",
        phase: phaseOf(row.run.doing),
        id: row.run.run.slice(0, 8),
        title: titleOf(row.run, lang),
        said: phaseSaid(lang, row.run.doing, now),
        bar: barOf(row.run, now),
      };
  }
}

// A run is titled by what the person asked of it, else by what
// finishing looks like; one dispatched with neither written down is
// named by where it works instead, so two such runs in one list are
// still two different lines.
function titleOf(run: BoardRun, lang: Lang): string {
  const words = [run.task, run.goal].find((said) => said !== null && said.trim() !== "");
  if (words !== undefined && words !== null) return words;
  return run.addr === null ? say(lang, "runs_untitled") : fill(say(lang, "runs_untitled_in"), { addr: run.addr });
}

// The bar draws what the belief knows: a run's life runs from its start
// to its end (or now), and its phase is known only as it stands, so the
// life is one quiet stretch and the phase is the cap at its right-hand
// end.
function barOf(run: BoardRun, now: number): { readonly left: number; readonly width: number } {
  const from = along(run.started ?? now, now);
  const to = along(run.ended ?? now, now);
  return { left: from, width: Math.max(to - from, 0) };
}
