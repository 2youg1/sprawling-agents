// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A slider over a whole-number range with its figure beside it, as the
// appearance group's blend card draws one. The bag the look spreads on
// the `<input type="range">` is built here, so the range, the accessible
// name and the figure read out stay with the seat whatever draws them;
// the look is `slider.look.svelte`.

// The bag spread on the range input. Arrow keys, Home, End and Page
// keys are the platform's own on a range input, so the bag carries no
// key table.
export interface RangeWire {
  readonly type: "range";
  readonly min: number;
  readonly max: number;
  readonly step: number;
  readonly value: number;
  readonly "aria-label": string;
  // The figure as a person reads it; `undefined` while nobody has
  // stated one, so the platform reads the bare number.
  readonly "aria-valuetext": string | undefined;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface SliderLook {
  readonly wire: RangeWire;
  // The figure drawn beside the track; empty while nobody has stated one.
  readonly shown: string;
}

export interface Range {
  readonly min: number;
  readonly max: number;
  readonly step: number;
}

export interface SliderProps {
  // The accessible name, already in the person's language.
  readonly label: string;
  readonly range: Range;
  // Where the thumb stands; `null` while nobody has stated a figure and
  // the page drew none, when the thumb rests at the range's start.
  readonly at: number | null;
  // How a figure reads, such as `40%`.
  readonly figure: (at: number) => string;
  // The raw value the thumb moved to; parsing it is the caller's.
  readonly onMove: (moved: string) => void;
}

export function sliderOf(props: SliderProps): SliderLook {
  const { range, at } = props;
  const said = at === null ? undefined : props.figure(at);
  return {
    shown: said ?? "",
    wire: {
      type: "range",
      min: range.min,
      max: range.max,
      step: range.step,
      value: at ?? range.min,
      "aria-label": props.label,
      "aria-valuetext": said,
      oninput: (event) => {
        props.onMove(event.currentTarget.value);
      },
    },
  };
}
