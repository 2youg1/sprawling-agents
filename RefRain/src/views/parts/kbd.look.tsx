// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";

import type { KbdLook } from "./kbd";

export function KbdMarks(props: KbdLook) {
  return (
    <span class={["inline-flex items-center gap-tight", props.class]}>
      <For each={props.marks}>
        {(mark) => (
          <kbd class="rounded-control px-tight font-mono text-note leading-none text-text-quiet no-underline">
            {mark}
          </kbd>
        )}
      </For>
    </span>
  );
}
