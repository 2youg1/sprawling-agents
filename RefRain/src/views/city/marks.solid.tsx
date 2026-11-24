// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Match, Show, Switch } from "solid-js";
import type { MarkLook } from "./skyline";

export default function Mark(props: { readonly placed: MarkLook }) {
  const figure = () => {
    const mark = props.placed;
    return mark.kind === "figure" ? mark : null;
  };
  const flag = () => {
    const mark = props.placed;
    return mark.kind === "flag" ? mark : null;
  };
  return (
    <Switch>
      <Match when={figure()}>
        {(placed) => {
          const tone = () => (placed().posture === "waiting" ? "fill-alert" : "fill-drawn-figure");
          return (
            <g
              class="bob"
              style={{
                "animation-delay": `${placed().delay}ms`,
                "transform-origin": `${placed().x}px ${placed().y}px`,
              }}
              aria-label={placed().label}
            >
              <path d={`M${placed().x - 5} ${placed().y} q5 -13 10 0 z`} class={tone()} />
              <circle cx={placed().x} cy={placed().y - 16} r="4.2" class={tone()} />
              <Show when={placed().posture === "thinking"}>
                <g class="blink">
                  <circle
                    cx={placed().x + 7}
                    cy={placed().y - 24}
                    r="1.2"
                    class="fill-drawn-aside"
                  />
                  <circle
                    cx={placed().x + 10.5}
                    cy={placed().y - 28}
                    r="1.6"
                    class="fill-drawn-aside"
                  />
                  <circle
                    cx={placed().x + 15}
                    cy={placed().y - 33}
                    r="2.2"
                    class="fill-drawn-aside"
                  />
                </g>
              </Show>
              <Show when={placed().posture === "calling"}>
                <rect
                  x={placed().x + 5.5}
                  y={placed().y - 11}
                  width="6"
                  height="6"
                  rx="1.2"
                  class="fill-accent"
                />
              </Show>
              <Show when={placed().posture === "waiting"}>
                <circle
                  cx={placed().x}
                  cy={placed().y - 16}
                  r="8"
                  class="fill-none stroke-alert"
                  stroke-width="1.4"
                />
              </Show>
            </g>
          );
        }}
      </Match>
      <Match when={flag()}>
        {(placed) => (
          <g aria-label={placed().label}>
            <line
              x1={placed().x}
              y1={placed().y - 22}
              x2={placed().x}
              y2={placed().y}
              class="stroke-drawn-stem"
              stroke-width="1"
            />
            <path
              d={`M${placed().x} ${placed().y - 22} l16 4 l-16 5 z`}
              class={placed().running ? "fill-accent wave" : "fill-drawn-part"}
              style={{ "transform-origin": `${placed().x}px ${placed().y - 18}px` }}
            />
          </g>
        )}
      </Match>
    </Switch>
  );
}
