// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The two marks the skyline carries that are about one run and one
// goal rather than about the building: a figure at the door for each
// run still moving, and a flag on the roof where a pursuit stands. The
// legend beside the drawing names both.
//
// Every element here is one Solid knows is SVG by its name, because the
// `<svg>` these are drawn into is in another file (client-SPEC D47).

import { Show } from "solid-js";

import type { RunBelief } from "../../core/belief";
import type { PursuitLine } from "../../wire";

export function Figure(props: { readonly run: RunBelief; readonly x: number; readonly y: number; readonly delay: number }) {
  const posture = () => props.run.doing.kind;
  const tone = () => (posture() === "waiting" ? "fill-alert" : "fill-g9");
  return (
    <g
      class="bob"
      style={{ "animation-delay": `${String(props.delay)}ms`, "transform-origin": `${String(props.x)}px ${String(props.y)}px` }}
      aria-label={`${props.run.addr ?? ""} · ${posture()}`}
    >
      <path d={`M${String(props.x - 5)} ${String(props.y)} q5 -13 10 0 z`} class={tone()} />
      <circle cx={props.x} cy={props.y - 16} r="4.2" class={tone()} />
      <Show when={posture() === "thinking"}>
        <g class="blink">
          <circle cx={props.x + 7} cy={props.y - 24} r="1.2" class="fill-g7" />
          <circle cx={props.x + 10.5} cy={props.y - 28} r="1.6" class="fill-g7" />
          <circle cx={props.x + 15} cy={props.y - 33} r="2.2" class="fill-g7" />
        </g>
      </Show>
      <Show when={posture() === "calling"}>
        <rect x={props.x + 5.5} y={props.y - 11} width="6" height="6" rx="1.2" class="fill-accent" />
      </Show>
      <Show when={posture() === "waiting"}>
        <circle cx={props.x} cy={props.y - 16} r="8" class="fill-none stroke-alert" stroke-width="1.4" />
      </Show>
    </g>
  );
}

export function Flag(props: { readonly x: number; readonly y: number; readonly line: PursuitLine }) {
  const running = () => props.line.state === "running";
  return (
    <g aria-label={props.line.goal}>
      <line x1={props.x} y1={props.y - 22} x2={props.x} y2={props.y} class="stroke-g6" stroke-width="1" />
      <path
        d={`M${String(props.x)} ${String(props.y - 22)} l16 4 l-16 5 z`}
        class={running() ? "fill-accent wave" : "fill-g5"}
        style={{ "transform-origin": `${String(props.x)}px ${String(props.y - 18)}px` }}
      />
    </g>
  );
}
