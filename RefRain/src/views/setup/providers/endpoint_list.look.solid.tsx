// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { For, Show } from "solid-js";
import Badge from "../../parts/badge.solid";
import Empty from "../../parts/empty.solid";
import type { EndpointListLook } from "./endpoint_list";

export interface EndpointListProps extends EndpointListLook {
  readonly accounts: (name: string) => JSX.Element;
  readonly models: (name: string) => JSX.Element;
}

export default function EndpointListView(props: EndpointListProps) {
  return (
    <Show
      when={props.rows.length > 0}
      fallback={<Empty missing="setup_no_endpoints" seat="region" />}
    >
      <ul class="flex flex-col gap-snug">
        <For each={props.rows} keyed={(row) => row.name}>
          {(row) => (
            <li class="rounded-card bg-chrome px-base py-snug text-note">
              <div class="flex min-w-0 items-center gap-snug">
                <span class="font-label text-text">{row().label}</span>
                <Show when={row().id}>
                  {(id) => <span class="font-mono text-text-faint">{id()}</span>}
                </Show>
                <Show when={row().local}>{(local) => <Badge text={local()} />}</Show>
                <span class="text-text-faint">{row().face}</span>
                <span class="min-w-0 flex-1 truncate font-mono text-text-faint">
                  {row().baseUrl}
                </span>
                <span class="shrink-0 text-text-faint">{row().keyed}</span>
              </div>
              <Show when={row().models.length > 0}>
                <div class="mt-tight flex flex-wrap gap-tight text-text-quiet">
                  <For each={row().models}>
                    {(model) => (
                      <span class="rounded-pill bg-raised px-snug font-mono">{model}</span>
                    )}
                  </For>
                </div>
              </Show>
              <details class="mt-snug">
                <summary
                  data-control=""
                  class="w-fit cursor-pointer rounded-control px-tight text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive"
                >
                  {row().accounts}
                </summary>
                <div class="pt-snug">{props.accounts(row().name)}</div>
              </details>
              {props.models(row().name)}
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}
