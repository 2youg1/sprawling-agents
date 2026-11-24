// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, For, Show } from "solid-js";

import Mark from "./marks.solid";
import type { Pane, SkylineLook } from "./skyline";

const PLINTH = 14;
const HALL_COLUMNS = [0, 1, 2, 3] as const;
const PANE: Record<Pane, string> = {
  dark: "fill-drawn-hollow ease-leave",
  ended: "fill-drawn-line ease-leave",
  waiting: "fill-alert ease-arrive",
  calling: "fill-accent-solid blink ease-arrive",
  lit: "fill-accent-solid ease-arrive",
};

export default function SkylineLookView(look: SkylineLook) {
  const uid = createUniqueId();
  const glow = `${uid}-glow`;
  const ground = `${uid}-ground`;
  return (
    <svg
      viewBox={`0 0 ${look.width} ${look.height}`}
      class="block w-full"
      style={{ "aspect-ratio": `${look.width} / ${look.height}`, "max-width": `${look.width}px` }}
      role="img"
      aria-label={look.label}
    >
      <defs>
        <radialGradient id={glow} cx="50%" cy="60%" r="60%">
          <stop offset="0%" class="[stop-color:var(--color-accent)]" stop-opacity="0.16" />
          <stop offset="100%" class="[stop-color:var(--color-accent)]" stop-opacity="0" />
        </radialGradient>
        <linearGradient id={ground} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" class="[stop-color:var(--color-drawn-solid)]" />
          <stop offset="100%" class="[stop-color:var(--color-drawn-hollow)]" />
        </linearGradient>
      </defs>
      <For each={look.stars} keyed={(item) => item.slot}>
        {(star) => (
          <>
            <circle
              cx={star().x}
              cy={star().y}
              r={star().r}
              class="fill-drawn-part blink"
              style={{ "animation-delay": `${star().delay}ms`, "animation-duration": "2.6s" }}
            />
          </>
        )}
      </For>
      <rect
        x="0"
        y={look.ground}
        width={look.width}
        height={look.height - look.ground}
        fill={`url(#${ground})`}
      />
      <line
        x1="0"
        y1={look.ground}
        x2={look.width}
        y2={look.ground}
        class="stroke-drawn-line"
        stroke-width="1"
      />
      <For each={look.towers} keyed={(item) => item.key}>
        {(tower) => (
          <>
            <g
              class="group cursor-pointer"
              role={tower().wire.role}
              tabindex={tower().wire.tabindex}
              aria-label={tower().wire["aria-label"]}
              onClick={() => tower().wire.onclick()}
              onKeyDown={(event) => tower().wire.onkeydown(event)}
            >
              <rect
                x={tower().x - 16}
                y={tower().top - 16}
                width={tower().w + 32}
                height={tower().h + 16}
                rx="24"
                fill={`url(#${glow})`}
                class={[
                  "transition-opacity",
                  tower().lit ? "opacity-100 ease-arrive" : "opacity-0 ease-leave",
                ]}
              />
              <Show when={tower().hall}>
                <path
                  d={`M${tower().x + tower().w / 2 - 34} ${tower().top} a34 26 0 0 1 68 0 z`}
                  class="fill-drawn-solid-lit stroke-drawn-edge"
                  stroke-width="1"
                />
                <line
                  x1={tower().x + tower().w / 2}
                  y1={tower().top - 26}
                  x2={tower().x + tower().w / 2}
                  y2={tower().top - 38}
                  class="stroke-drawn-stem"
                  stroke-width="1.2"
                />
                <circle
                  cx={tower().x + tower().w / 2}
                  cy={tower().top - 40}
                  r="2"
                  class="fill-drawn-aside"
                />
              </Show>
              <path
                d={tower().outline}
                class={[
                  tower().hall ? "fill-drawn-solid-lit" : "fill-drawn-solid",
                  tower().picked ? "stroke-accent" : "stroke-drawn-line",
                  "transition-colors ease-leave group-hover:fill-drawn-line group-hover:ease-arrive",
                  "group-focus-visible:fill-drawn-line group-focus-visible:ease-arrive",
                ]}
                stroke-width={tower().picked ? "2" : "1"}
              />
              <line
                x1={tower().x + 8}
                y1={tower().top + 10}
                x2={tower().x + tower().w - 8}
                y2={tower().top + 10}
                class="stroke-drawn-line"
                stroke-width="1"
              />
              <For each={tower().cells} keyed={(item) => item.slot}>
                {(cell) => (
                  <>
                    <rect
                      x={cell().x}
                      y={cell().y}
                      width="12"
                      height="14"
                      rx="1.5"
                      class={["transition-colors", PANE[cell().pane]]}
                    />
                  </>
                )}
              </For>
              <Show when={tower().hall}>
                <For each={HALL_COLUMNS}>
                  {(column) => (
                    <>
                      <rect
                        x={tower().x + tower().w / 2 - 42 + column * 28 - 2.5}
                        y={look.ground - 46}
                        width="5"
                        height="46"
                        rx="1"
                        class="fill-drawn-line"
                      />
                    </>
                  )}
                </For>
              </Show>
              <rect
                x={tower().door.x}
                y={tower().door.y}
                width={tower().door.w}
                height={tower().door.h}
                rx="2"
                class="fill-drawn-hollow"
              />
              <rect
                x={tower().door.x + 2}
                y={tower().door.y + 2}
                width={tower().door.w - 4}
                height="4"
                rx="1"
                class={[
                  "transition-colors",
                  tower().lit ? "fill-accent-solid ease-arrive" : "fill-drawn-solid-lit ease-leave",
                ]}
              />
              <Show when={tower().lamp}>
                {(lamp) => (
                  <>
                    <g class="blink" aria-label={lamp()}>
                      <line
                        x1={tower().door.x - 10}
                        y1={look.ground}
                        x2={tower().door.x - 10}
                        y2={look.ground - 26}
                        class="stroke-drawn-part"
                        stroke-width="1"
                      />
                      <circle
                        cx={tower().door.x - 10}
                        cy={look.ground - 28}
                        r="3"
                        class="fill-alert"
                      />
                    </g>
                  </>
                )}
              </Show>
              <rect
                x={tower().x}
                y={look.ground + 2}
                width={tower().w}
                height={PLINTH}
                rx="2"
                class="fill-drawn-solid"
              />
              <Show when={tower().bar}>
                {(bar) => (
                  <>
                    <rect
                      x={tower().x + 6}
                      y={look.ground + 7}
                      width={tower().w - 12}
                      height="3"
                      rx="1.5"
                      class="fill-drawn-line"
                    />
                    <rect
                      x={tower().x + 6}
                      y={look.ground + 7}
                      width={(tower().w - 12) * bar().done}
                      height="3"
                      rx="1.5"
                      class="fill-accent"
                    />
                    <rect
                      x={tower().x + 6 + (tower().w - 12) * bar().done}
                      y={look.ground + 7}
                      width={(tower().w - 12) * bar().blocked}
                      height="3"
                      rx="1.5"
                      class="fill-alert"
                    />
                  </>
                )}
              </Show>
              <text
                x={tower().x + tower().w / 2}
                y={look.ground + PLINTH + 22}
                text-anchor="middle"
                class={[
                  "font-mono transition-colors ease-leave group-hover:fill-text group-hover:ease-arrive",
                  "group-focus-visible:fill-text group-focus-visible:ease-arrive",
                  tower().picked ? "fill-text" : "fill-text-quiet",
                ]}
                font-size={String(tower().name.size)}
              >
                <title>{tower().name.whole}</title>
                {tower().name.shown}
              </text>
              <Show when={tower().line}>
                {(line) => (
                  <>
                    <text
                      x={tower().x + tower().w / 2}
                      y={look.ground + PLINTH + 38}
                      text-anchor="middle"
                      class="fill-text-disabled font-mono"
                      font-size={String(line().size)}
                    >
                      <title>{line().whole}</title>
                      {line().shown}
                    </text>
                  </>
                )}
              </Show>
              <For each={tower().marks} keyed={false}>
                {(placed) => (
                  <>
                    <Mark placed={placed()} />
                  </>
                )}
              </For>
            </g>
          </>
        )}
      </For>
    </svg>
  );
}
