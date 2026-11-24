// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { lookOf } from "./reasoning";

export interface ReasoningProps {
  readonly text: string;
  readonly open: boolean;
}

export default function Reasoning(props: ReasoningProps) {
  const lang = storeValue(ui().lang);
  const [shown, setShown] = createSignal<string | null>(null);
  createEffect(
    () => props.text,
    (next) => {
      const frame = requestAnimationFrame(() => setShown(next));
      return () => {
        cancelAnimationFrame(frame);
      };
    },
  );
  const look = createMemo(() => lookOf(lang(), shown() ?? props.text, props.open));
  return (
    <details class="text-note text-text-faint" open={look().open}>
      <summary class="cursor-pointer rounded-control px-tight marker:text-text-faint hover:wash hover:text-text-quiet">
        <span class="text-text-faint">{look().label}</span>
        {look().length}
      </summary>
      <div class="mt-tight border-l border-edge-panel pl-base whitespace-pre-wrap break-words">
        {look().text}
      </div>
    </details>
  );
}
