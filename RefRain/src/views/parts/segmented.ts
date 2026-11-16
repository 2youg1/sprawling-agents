// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

export type Tone = "plain" | "alert" | "accent";

export interface Group {
  readonly label: string;
  readonly tone: Tone;
}

export interface Choice<V extends string> {
  readonly value: V;

  readonly label: string;
  readonly group?: Group;
  readonly why?: string;
}

export interface SegmentedProps<V extends string> {
  readonly label: string;
  readonly options: readonly Choice<V>[];

  readonly held: V | null;
  readonly onPick: (value: V) => void;

  readonly tone?: Tone;
}

export interface Band<V extends string> {
  readonly group: Group | undefined;
  readonly from: number;
  readonly cells: readonly Choice<V>[];
}

export function bands<V extends string>(options: readonly Choice<V>[]): readonly Band<V>[] {
  const out: { group: Group | undefined; from: number; cells: Choice<V>[] }[] = [];
  options.forEach((choice, at) => {
    const open = out.at(-1);
    if (open !== undefined && alike(open.group, choice.group)) {
      open.cells.push(choice);
      return;
    }
    out.push({ group: choice.group, from: at, cells: [choice] });
  });
  return out;
}

function alike(held: Group | undefined, next: Group | undefined): boolean {
  if (held === undefined || next === undefined) return held === next;
  return held.label === next.label && held.tone === next.tone;
}

export function nextStop<V extends string>(
  options: readonly Choice<V>[],
  from: number,
  step: 1 | -1,
): number {
  const total = options.length;
  if (total === 0) return from;
  let at = ((from % total) + total) % total;
  for (let taken = 0; taken < total; taken += 1) {
    at = (at + step + total) % total;
    if (options[at]?.why === undefined) return at;
  }
  return from;
}

export function tabStop<V extends string>(options: readonly Choice<V>[], held: V | null): number {
  const chosen = options.findIndex((choice) => choice.value === held);
  if (chosen >= 0) return chosen;
  const free = options.findIndex((choice) => choice.why === undefined);
  if (free >= 0) return free;
  return options.length > 0 ? 0 : -1;
}

export function landing<V extends string>(
  options: readonly Choice<V>[],
  at: number,
  key: string,
): number | undefined {
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
      return nextStop(options, at, 1);
    case "ArrowLeft":
    case "ArrowUp":
      return nextStop(options, at, -1);
    case " ":
      return at;
    default:
      return undefined;
  }
}

export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;

export interface GroupWire {
  readonly role: "radiogroup";
  readonly "aria-label": string;
}

export interface CellWire {
  readonly role: "radio";
  readonly "aria-checked": boolean;
  readonly "aria-disabled": boolean;
  readonly "aria-describedby": string | undefined;
  readonly tabindex: 0 | -1;
  readonly onclick: () => void;
  readonly onkeydown: (event: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface Why {
  readonly id: string;
  readonly text: string;
}

export type CellState = "held" | "free" | "refused";

export interface CellLook {
  readonly key: string;

  readonly label: string;
  readonly state: CellState;
  readonly why: Why | undefined;
  readonly wire: CellWire;
}

export interface BandLook {
  readonly key: number;

  readonly heading: string | undefined;

  readonly tone: Tone;

  readonly held: number;
  readonly cells: readonly CellLook[];
}

export interface SegmentedLook {
  readonly group: GroupWire;
  readonly bands: readonly BandLook[];
}

export interface Hands<V extends string> {
  readonly focus: (value: V) => void;
  readonly hold: (value: V) => Attachment<HTMLElement>;
}

const HOLD = createAttachmentKey();

export function lookOf<V extends string>(
  props: SegmentedProps<V>,
  uid: string,
  hands: Hands<V>,
): SegmentedLook {
  const { options, held } = props;
  const stop = tabStop(options, held);

  const choose = (at: number): void => {
    const choice = options[at];
    if (choice === undefined || choice.why !== undefined) return;
    props.onPick(choice.value);
    hands.focus(choice.value);
  };

  const cellOf = (choice: Choice<V>, at: number): CellLook => {
    const why =
      choice.why === undefined ? undefined : { id: `${uid}-why-${String(at)}`, text: choice.why };
    return {
      key: choice.value,
      label: choice.label,
      state: stateOf(choice, held),
      why,
      wire: {
        role: "radio",
        "aria-checked": choice.value === held,
        "aria-disabled": why !== undefined,
        "aria-describedby": why?.id,
        tabindex: at === stop ? 0 : -1,
        onclick: () => {
          choose(at);
        },
        onkeydown: (event) => {
          const to = landing(options, at, event.key);
          if (to === undefined) return;
          event.preventDefault();
          choose(to);
        },
        [HOLD]: hands.hold(choice.value),
      },
    };
  };

  return {
    group: { role: "radiogroup", "aria-label": props.label },
    bands: bands(options).map((band) => ({
      key: band.from,
      heading: band.group?.label,
      tone: band.group?.tone ?? props.tone ?? "plain",
      held: band.cells.findIndex((choice) => choice.value === held),
      cells: band.cells.map((choice, inBand) => cellOf(choice, band.from + inBand)),
    })),
  };
}

function stateOf<V extends string>(choice: Choice<V>, held: V | null): CellState {
  if (choice.why !== undefined) return "refused";
  return choice.value === held ? "held" : "free";
}
