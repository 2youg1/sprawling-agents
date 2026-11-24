// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import "./marks.css";

export interface TypedLineProps {
  readonly text: string;
  readonly box: HTMLTextAreaElement | undefined;
  readonly lit: boolean;
}

export function TypedLineLook(props: { readonly typed: number; readonly lit: boolean }) {
  return (
    <span
      class="typed-line"
      style={{ "--typed": `${String(props.typed)}px` }}
      data-lit={props.lit ? "" : undefined}
      aria-hidden="true"
    />
  );
}

export default function TypedLine(props: TypedLineProps) {
  let ruler: CanvasRenderingContext2D | null | undefined;
  const width = createMemo(() => {
    const box = props.box;
    const words = props.text;
    if (box === undefined || words === "") return 0;
    ruler ??= document.createElement("canvas").getContext("2d");
    if (ruler === null) return box.clientWidth;
    ruler.font = getComputedStyle(box).font;
    const pen = ruler;
    const farthest = Math.max(...words.split("\n").map((line) => pen.measureText(line).width));
    return Math.min(farthest + 2, box.clientWidth);
  });
  return <TypedLineLook typed={width()} lit={props.lit} />;
}
