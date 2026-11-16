// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface RangeWire {
  readonly type: "range";
  readonly min: number;
  readonly max: number;
  readonly step: number;
  readonly value: number;
  readonly "aria-label": string;
  readonly "aria-valuetext": string | undefined;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface SliderLook {
  readonly wire: RangeWire;
  readonly shown: string;
}

export interface Range {
  readonly min: number;
  readonly max: number;
  readonly step: number;
}

export interface SliderProps {
  readonly label: string;
  readonly range: Range;
  readonly at: number | null;
  readonly figure: (at: number) => string;
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
