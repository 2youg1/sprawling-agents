// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";

import type { TokenLook } from "./token";

export default function TokenView(props: TokenLook) {
  return (
    <li class="flex items-center gap-tight">
      <input
        type={props.well.type}
        aria-label={props.well["aria-label"]}
        value={props.well.value}
        onChange={(event) => props.well.onchange(event)}
        class="h-control w-control shrink-0 rounded-control border border-edge-input bg-raised"
      />
      <span class="min-w-0 flex-1 truncate font-mono text-note text-text-quiet">{props.name}</span>
      <Show when={props.reset}>
        {(reset) => (
          <button
            data-control=""
            type={reset().wire.type}
            aria-label={reset().wire["aria-label"]}
            onClick={() => reset().wire.onclick()}
            class="h-control-sm shrink-0 rounded-control px-snug text-note text-text-quiet transition-colors duration-short ease-leave hover:bg-raised-hover hover:text-text hover:ease-arrive"
          >
            {reset().word}
          </button>
        )}
      </Show>
    </li>
  );
}

export function TokenList(props: { readonly tokens: readonly TokenLook[] }) {
  return (
    <ul class="flex flex-col gap-tight">
      <For each={props.tokens} keyed={(token) => token.name}>
        {(token) => <TokenView {...token()} />}
      </For>
    </ul>
  );
}
