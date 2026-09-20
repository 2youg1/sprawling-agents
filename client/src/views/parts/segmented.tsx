// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One exclusive choice among a few, drawn as a track with a slider on
// it. It replaces the pill rows the settings pages grew one at a time,
// each of which announced itself as a row of unrelated pressed buttons.
//
// Three things this control owes a person that a row of pills does not
// give them. A screen reader hears one question with several answers,
// because the track is a `radiogroup` and every cell reports whether it
// is the chosen one. A keyboard crosses the whole control in two keys,
// because only the chosen cell is a tab stop and the arrows move
// between cells. And a cell that cannot be chosen says why - through
// `Tip`, so the reason reaches a pointer, a keyboard and a touch screen
// alike, and arrives before the click rather than as a refusal after
// it.
//
// The slider travels on `transform` over cells of one width, so the
// move is a compositor job and no script measures anything. The one
// stylesheet decides whether it travels at all.
//
// The words are the caller's: this file holds no prose.

import { For, Show, createSignal } from "solid-js";
import type { JSX } from "solid-js";

import { Tip } from "./tip";

// Which of the two coloured tokens paints the chosen cell. It is a
// caller's decision and never an inference: this control knows nothing
// about what its values mean.
export type Tone = "accent" | "alert";

const FILL: Record<Tone, string> = {
  accent: "bg-accent",
  alert: "bg-alert",
};

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

// The slider's geometry, written once because neither half depends on
// which band draws it: the width is one cell of however many the track
// holds, and the travel is that width times the cell that is chosen.
// The track states both numbers as custom properties and the slider
// inherits them, so a change of choice moves one value.
const SLIDER = {
  width: "calc(100% / var(--cells))",
  transform: "translateX(calc(var(--held) * 100%))",
};

export function Segmented<V extends string>(props: SegmentedProps<V>) {
  const [cells, setCells] = createSignal<readonly HTMLButtonElement[]>([]);
  const stop = () => tabStop(props.options, props.held);

  const hold = (at: number, cell: HTMLButtonElement) => {
    setCells((before) => {
      const next = [...before];
      next[at] = cell;
      return next;
    });
  };

  const move = (step: 1 | -1) => {
    const to = nextStop(props.options, stop(), step);
    const choice = props.options[to];
    if (choice === undefined) return;
    props.onPick(choice.value);
    cells()[to]?.focus();
  };

  const travel = (event: KeyboardEvent) => {
    switch (event.key) {
      case "ArrowRight":
        event.preventDefault();
        move(1);
        return;
      case "ArrowLeft":
        event.preventDefault();
        move(-1);
        return;
      default:
        return;
    }
  };

  const ink = (choice: Choice<V>): string => {
    if (choice.why !== undefined) return "text-text-disabled";
    if (choice.value === props.held) return "text-g0";
    return "text-text-quiet hover:text-text";
  };

  return (
    <div
      class="inline-flex items-end gap-snug"
      role="radiogroup"
      aria-label={props.label}
      onKeyDown={travel}
    >
      <For each={bands(props.options)}>
        {(band, at) => {
          const here = () => band.cells.findIndex((choice) => choice.value === props.held);
          const tone = () => band.group?.tone ?? props.tone ?? "accent";
          return (
            <>
              <Show when={at() > 0}>
                <span aria-hidden="true" class="w-hair self-stretch bg-g3" />
              </Show>
              <div class="flex flex-col gap-tight">
                <Show when={band.group}>
                  {(group) => <span class="px-base text-note text-text-quiet">{group().label}</span>}
                </Show>
                <div
                  class="relative grid auto-cols-fr grid-flow-col rounded-pill bg-g1"
                  style={{
                    "--cells": String(band.cells.length),
                    "--held": String(Math.max(here(), 0)),
                  }}
                >
                  <Show when={here() >= 0}>
                    <span
                      aria-hidden="true"
                      class={`pointer-events-none absolute inset-y-0 left-0 rounded-pill transition-transform ease-standard motion-reduce:transition-none ${FILL[tone()]}`}
                      style={SLIDER}
                    />
                  </Show>
                  <For each={band.cells}>
                    {(choice, inBand) => {
                      const at = () => band.from + inBand();
                      // One definition of the cell, drawn bare or
                      // inside its reason. `Tip` wraps the cell in a
                      // box of its own, so the cell takes the whole
                      // grid column and the slider still travels one
                      // column at a time.
                      const cell = (hint?: string): JSX.Element => (
                        <button
                          ref={(node) => {
                            hold(at(), node);
                          }}
                          type="button"
                          role="radio"
                          aria-checked={choice.value === props.held}
                          aria-disabled={choice.why !== undefined}
                          aria-describedby={hint}
                          tabindex={at() === stop() ? 0 : -1}
                          class={`relative w-full rounded-pill px-base py-tight text-label whitespace-nowrap ${ink(choice)}`}
                          onClick={() => {
                            if (choice.why !== undefined) return;
                            props.onPick(choice.value);
                          }}
                        >
                          {choice.label}
                        </button>
                      );
                      return (
                        <Show when={choice.why} fallback={cell()}>
                          {(why) => <Tip text={why()}>{(hint) => cell(hint)}</Tip>}
                        </Show>
                      );
                    }}
                  </For>
                </div>
              </div>
            </>
          );
        }}
      </For>
    </div>
  );
}
