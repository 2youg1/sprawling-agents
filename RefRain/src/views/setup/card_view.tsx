// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { Show } from "solid-js";

import Glyph from "../parts/glyph.solid";
import type { CardLook } from "./card";

interface CardProps extends Omit<CardLook, "children"> {
  readonly children: JSX.Element;
}

export default function Card(props: CardProps) {
  return (
    <div class="flex min-w-0 flex-col gap-snug rounded-card bg-raised px-base py-snug">
      <div class="flex flex-col gap-hair">
        <span class="text-label font-label text-text">{props.title}</span>
        <Show when={props.note !== undefined}>
          <p class="text-note text-text-faint">{props.note}</p>
        </Show>
      </div>
      {props.children}
      <Show when={props.constraint !== undefined || props.receipt !== undefined}>
        <div class="flex items-baseline justify-between gap-base">
          <Show when={props.constraint !== undefined}>
            <span class="text-note text-text-faint">{props.constraint}</span>
          </Show>
          <Show when={props.receipt}>
            {(receipt) => (
              <span
                {...receipt().wire}
                class="ml-auto inline-flex items-center gap-tight text-note text-text-quiet"
              >
                <Show when={receipt().said !== undefined}>
                  <span class="fade inline-flex items-center gap-tight">
                    <Glyph name="check" size="sm" class="shrink-0" />
                    {receipt().said}
                  </span>
                </Show>
              </span>
            )}
          </Show>
        </div>
      </Show>
    </div>
  );
}
