// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The properties this module must hold are proved in `client/spec/Views/Parts/Segmented.lean`.
//
// What the segmented control decides before anything is drawn: the
// vocabulary a track is built from, where a track begins and ends,
// where an arrow key lands, and which cell a keyboard arrives at. The
// drawing is `segmented.svelte`; this module touches no DOM, so these
// decisions are testable without compiling a component (`bun test`
// cannot compile Svelte). Where a box landed is not something a unit
// test can be told: `cargo xtask render` measures that in a real
// engine against the gallery fixtures.

// What paints the chosen cell. It is a caller's decision and never an
// inference: this control knows nothing about what its values mean.
//
// **`plain` is the default, and that is a change of policy.** The
// slider used to be filled with the accent, which made "which tab is
// selected" the loudest mark on any page carrying a segmented control
// - louder than the stop button, and louder than anything the model
// had just written. The accent is a budget: it buys the focus ring and
// the bar beside a selected row, and nothing else. Selection is said
// here the way it is said everywhere else on this page, by lifting the
// surface a step.
//
// `alert` survives because a choice that widens what a run may do is
// not the same kind of fact as a choice of units, and a person has to
// be able to see which one they are looking at across the room.
export type Tone = "plain" | "alert";

// A heading over a run of cells, with the tone that run is painted in.
// Cells carrying an equal group are drawn as one track, and a change of
// group draws a rule between the tracks.
export interface Group {
  // Already in the person's language.
  readonly label: string;
  readonly tone: Tone;
}

// One cell of the track.
//
// **`why` present means the cell cannot be chosen, and says why.** The
// two are one fact and therefore one field: a cell disabled without a
// reason is a dead end, and a reason on a cell a person can choose is
// never read. This is the reading `parts/button.tsx` already gives the
// same field.
export interface Choice<V extends string> {
  readonly value: V;
  // Already in the person's language.
  readonly label: string;
  readonly group?: Group;
  readonly why?: string;
}

export interface SegmentedProps<V extends string> {
  // The accessible name of the question, already in the person's
  // language. A radiogroup without one is announced as a group of
  // radios and nothing else.
  readonly label: string;
  readonly options: readonly Choice<V>[];
  // `null` is nobody has chosen yet, which the caller may hold and this
  // control draws: no cell reports itself chosen, no slider is drawn,
  // and the tab stop falls to the first cell that can be chosen. A
  // seventh cell or an empty string standing for absence would give
  // "nobody said" a second spelling, so absence is spelled once.
  readonly held: V | null;
  readonly onPick: (value: V) => void;
  // The tone of every cell that states no group of its own; a group
  // overrides it for the cells under it.
  readonly tone?: Tone;
}

// A run of neighbouring cells that share one group, with the position
// of its first cell in the flat list the caller gave.
export interface Band<V extends string> {
  readonly group: Group | undefined;
  readonly from: number;
  readonly cells: readonly Choice<V>[];
}

// Cut the flat list into the tracks the control draws.
//
// Neighbouring cells join one band when their groups agree in both
// name and tone. Two spellings of one group therefore draw two tracks,
// which a person sees, rather than one track under one of the two
// tones, which nobody can tell from the intended drawing.
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

// The cell an arrow key moves to, counted from `from` in `step`.
//
// Cells that cannot be chosen are stepped over, because a radiogroup
// chooses what it focuses: landing on one would choose it. The walk
// wraps once around and answers `from` when nothing else can be
// chosen, so a control whose every cell is refused stays where it is.
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

// The one cell that is a tab stop, so the control costs a keyboard one
// stop rather than one per cell.
//
// It is the chosen cell. A control whose held value is not among its
// cells - including one where nobody has chosen - offers the first
// choosable one instead, and a control where every cell is refused
// offers its first cell, so that the reason on it can still be
// reached. An empty control offers nothing.
export function tabStop<V extends string>(options: readonly Choice<V>[], held: V | null): number {
  const chosen = options.findIndex((choice) => choice.value === held);
  if (chosen >= 0) return chosen;
  const free = options.findIndex((choice) => choice.why === undefined);
  if (free >= 0) return free;
  return options.length > 0 ? 0 : -1;
}
