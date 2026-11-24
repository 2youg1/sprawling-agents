// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createMemo, Show } from "solid-js";
import { lookOf, walk, type RowProps } from "./row";

const TEXT = "flex min-w-0 flex-1 flex-col text-left";

export function RowList(props: { readonly label: string; readonly children: JSX.Element }) {
  return (
    <ul aria-label={props.label} onKeyDown={walk}>
      {props.children}
    </ul>
  );
}

export default function Row(props: RowProps<JSX.Element>) {
  const look = createMemo(() => lookOf(props));
  const texts = () => (
    <>
      <span class="truncate text-body text-text">{look().primary}</span>
      <Show when={look().secondary !== undefined}>
        <span class="summary truncate text-note text-text-faint">{look().secondary}</span>
      </Show>
    </>
  );
  return (
    <li
      {...look().item}
      class={[
        "flex min-h-touch w-full min-w-0 items-center gap-base border-b border-edge px-base py-snug",
        "transition-[background-color] ease-leave hover:wash hover:ease-arrive has-[:focus-visible]:wash has-[:focus-visible]:ease-arrive",
        look().chosen && "chosen",
      ]}
    >
      <Show when={look().open} fallback={<div class={TEXT}>{texts()}</div>}>
        {(open) => (
          <button data-control="" type={open().type} onClick={open().onclick} class={TEXT}>
            {texts()}
          </button>
        )}
      </Show>
      <Show when={look().status !== undefined}>
        <div class="shrink-0">{look().status}</div>
      </Show>
      <Show when={look().actions !== undefined}>
        <div class="flex shrink-0 items-center gap-tight">{look().actions}</div>
      </Show>
    </li>
  );
}
