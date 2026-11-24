// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Match, Switch } from "solid-js";

import { QUERIES } from "../../core/asking";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { clock } from "../../core/time";
import { ui } from "../../ui";
import type { Decision } from "../../wire";

export default function Decided(props: { readonly decided?: readonly Decision[] }) {
  const u = ui();
  const lang = storeValue(u.lang);
  const governance = storeValue(u.conn.asking.ask(QUERIES.governance));
  const words = (key: Key) => say(lang(), key);
  const newest = createMemo(() => {
    const held = governance();
    const shown =
      props.decided ??
      (held !== undefined && "governance" in held ? held.governance.decided : undefined);
    return shown === undefined ? undefined : [...shown].sort((a, b) => b.at - a.at);
  });
  return (
    <div class="flex flex-col gap-tight">
      <span class="text-note font-label text-text-quiet">{words("decided_title")}</span>
      <Switch>
        <Match when={newest() === undefined}>
          <p class="text-note text-text-faint">…</p>
        </Match>
        <Match when={newest()?.length === 0}>
          <p class="text-note text-text-faint">{words("decided_none")}</p>
        </Match>
        <Match when={(newest()?.length ?? 0) > 0}>
          <ul class="max-h-output overflow-auto text-note">
            <For each={newest()} keyed={(decision) => decision.item}>
              {(decision) => (
                <li class="flex items-baseline gap-base border-b border-edge py-tight">
                  <span class="shrink-0 text-text-faint">{clock(lang(), decision().at)}</span>
                  <span class="min-w-0 flex-1 truncate text-text-quiet">
                    {decision().cluster.detail}
                  </span>
                  <span
                    class={
                      decision().verdict === "allow"
                        ? "shrink-0 text-accent"
                        : "shrink-0 text-alert"
                    }
                  >
                    {words(decision().verdict === "allow" ? "decided_allow" : "decided_deny")}
                  </span>
                </li>
              )}
            </For>
          </ul>
        </Match>
      </Switch>
    </div>
  );
}
