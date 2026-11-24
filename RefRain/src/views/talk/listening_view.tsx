// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Address } from "../../wire";
import { sessionRunOf } from "./gauge";
import { listeningOf } from "./listening";

export interface ListeningProps {
  readonly room: Address;
}

export default function Listening(props: ListeningProps) {
  const u = ui();
  const lang = storeValue(u.lang);
  const belief = storeValue(u.conn.belief);
  const run = createMemo(() => sessionRunOf(belief(), props.room));
  const answer = storeValue(() => {
    const current = run();
    return current === undefined ? undefined : u.conn.asking.ask({ prefix: { run: current } });
  });
  const look = createMemo(() => {
    const prefix = answer();
    return listeningOf(lang(), {
      city: belief().city,
      room: props.room,
      segments: prefix !== undefined && "prefix" in prefix ? prefix.prefix.segments : [],
      told: run() !== undefined,
    });
  });
  return (
    <div class="px-base pb-snug text-note">
      <p class="pb-tight text-text-faint">{look().heading}</p>
      <dl class="grid grid-cols-[max-content_minmax(0,1fr)] gap-x-base gap-y-tight">
        <For each={look().listeners} keyed={(listener) => listener.key}>
          {(listener) => (
            <>
              <dt class="text-text-faint">{listener().word}</dt>
              <dd class="flex min-w-0 flex-col">
                <span class="flex min-w-0 items-baseline gap-base">
                  <Show when={listener().who}>
                    {(who) => <span class="truncate text-text">{who()}</span>}
                  </Show>
                  <Show when={listener().size}>
                    {(size) => <span class="figure shrink-0 text-text-faint">{size()}</span>}
                  </Show>
                  <Show when={listener().cut}>
                    {(cut) => <span class="shrink-0 text-alert">{cut()}</span>}
                  </Show>
                </span>
                <Show when={listener().sources}>
                  {(sources) => <span class="truncate text-text-faint">{sources()}</span>}
                </Show>
              </dd>
            </>
          )}
        </For>
      </dl>
      <Show when={look().note}>{(note) => <p class="pt-tight text-text-faint">{note()}</p>}</Show>
    </div>
  );
}
