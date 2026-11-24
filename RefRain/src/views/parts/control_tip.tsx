// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createSignal, Show } from "solid-js";

const PLACE = "absolute bottom-full left-1/2 -translate-x-1/2 mb-tight";
const PLACE_ANCHORED =
  "supports-[anchor-name:--a]:fixed " +
  "supports-[anchor-name:--a]:bottom-auto supports-[anchor-name:--a]:left-auto " +
  "supports-[anchor-name:--a]:translate-x-0 supports-[anchor-name:--a]:[position-anchor:var(--tip-anchor)] " +
  "supports-[anchor-name:--a]:[position-area:block-start] supports-[anchor-name:--a]:[position-try-fallbacks:flip-block]";
const RIGHT = "absolute left-full top-1/2 -translate-y-1/2 ml-base";
const RIGHT_ANCHORED =
  "supports-[anchor-name:--a]:fixed " +
  "supports-[anchor-name:--a]:top-auto supports-[anchor-name:--a]:left-auto " +
  "supports-[anchor-name:--a]:translate-y-0 supports-[anchor-name:--a]:[position-anchor:var(--tip-anchor)] " +
  "supports-[anchor-name:--a]:[position-area:inline-end] supports-[anchor-name:--a]:[position-try-fallbacks:flip-inline]";
const PAINT =
  "pointer-events-none z-(--layer-popover) w-max max-w-measure rounded-card border border-edge-panel bg-raised " +
  "px-snug py-tight text-note text-text shadow-float " +
  "transition-[opacity,display] transition-discrete delay-300 duration-panel ease-leave still:transition-none";
const SHOW =
  "group-hover/tip:block group-hover/tip:opacity-100 group-hover/tip:ease-arrive " +
  "group-focus-within/tip:block group-focus-within/tip:opacity-100 group-focus-within/tip:ease-arrive";

interface TipProps {
  readonly id: string;
  readonly text?: string | undefined;
  readonly side?: "above" | "right";
  readonly placing?: "by-engine" | "against-wrapper";
  readonly standing?: boolean;
  readonly exposable?: boolean;
  readonly children: JSX.Element;
}

export default function ControlTip(props: TipProps) {
  const [dismissed, setDismissed] = createSignal(false);
  const [hovered, setHovered] = createSignal(false);
  const [focused, setFocused] = createSignal(false);
  createEffect(
    () => props.text !== undefined && !props.standing && (hovered() || focused()),
    (shown) => {
      if (!shown) return;
      function hear(event: KeyboardEvent): void {
        if (event.key === "Escape") setDismissed(true);
      }
      window.addEventListener("keydown", hear);
      return () => window.removeEventListener("keydown", hear);
    },
  );
  return (
    <span
      class="group/tip relative inline-flex min-w-0 items-center [anchor-name:var(--tip-anchor)]"
      style={{ "--tip-anchor": `--tip-${props.id}` }}
      onMouseEnter={() => {
        setHovered(true);
        setDismissed(false);
      }}
      onMouseLeave={() => setHovered(false)}
      onFocusIn={() => {
        setFocused(true);
        setDismissed(false);
      }}
      onFocusOut={(event) => {
        if (
          !(event.relatedTarget instanceof Node) ||
          !event.currentTarget.contains(event.relatedTarget)
        )
          setFocused(false);
      }}
    >
      {props.children}
      <Show when={props.text !== undefined}>
        <span
          class={[
            props.side === "right" ? RIGHT : PLACE,
            props.placing !== "against-wrapper" &&
              (props.side === "right" ? RIGHT_ANCHORED : PLACE_ANCHORED),
            PAINT,
            props.standing ? "block opacity-100" : ["hidden opacity-0", !dismissed() && SHOW],
          ]}
          id={props.id}
          role="tooltip"
          data-exposable={props.exposable === true && !dismissed() ? "" : undefined}
        >
          {props.text}
        </span>
      </Show>
    </span>
  );
}
