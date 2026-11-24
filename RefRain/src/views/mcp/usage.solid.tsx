// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createMemo } from "solid-js";
import { readAnswer } from "../../core/answered";
import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { Query } from "../../wire";
import EmptyState from "../parts/empty.solid";
import Unanswered from "../parts/unanswered.solid";
import UsageExport from "../parts/usage_export.solid";
import UsageUses from "../parts/usage_uses.solid";

const QUESTION: Query = { mcp_usage: { server: null } };

export default function McpUsage() {
  const u = ui();
  const lang = storeValue(u.lang);
  const asked = storeValue(u.conn.asking.ask(QUESTION));
  const read = createMemo(() =>
    readAnswer(asked(), (held) => ("mcp_usage" in held ? held.mcp_usage.servers : undefined)),
  );
  const query = createMemo(() => {
    const value = read();
    return value.kind === "unavailable" ? value.query : undefined;
  });
  const rows = createMemo(() => {
    const value = read();
    return value.kind === "held" ? value.value : undefined;
  });
  return (
    <section class="flex min-w-0 flex-col gap-base">
      <div class="flex flex-wrap items-baseline justify-between gap-base">
        <h2 class="text-note text-text-faint">{say(lang(), "usage_mcp_title")}</h2>
        <UsageExport what="mcp" />
      </div>
      <Switch fallback={<p class="text-text-faint">…</p>}>
        <Match when={query()}>
          {(unavailable) => <Unanswered query={unavailable()} asked={QUESTION} />}
        </Match>
        <Match when={rows()?.length === 0}>
          <EmptyState missing="usage_mcp_none" seat="region" />
        </Match>
        <Match when={rows()}>
          {(servers) => (
            <ul class="flex flex-col">
              <For each={servers()} keyed={(server) => server.server ?? ""}>
                {(server) => (
                  <li class="flex min-w-0 flex-col gap-tight border-b border-edge py-snug">
                    <span class="flex items-baseline gap-base text-note">
                      <span class="font-mono text-text-quiet">
                        {server().server ?? say(lang(), "usage_removed")}
                      </span>
                      <Show when={server().server != null && !server().configured}>
                        <span class="text-text-faint">{say(lang(), "usage_unconfigured")}</span>
                      </Show>
                    </span>
                    <Show when={server().tools.length === 0}>
                      <p class="text-note text-text-faint">{say(lang(), "usage_never")}</p>
                    </Show>
                    <For each={server().tools} keyed={(tool) => tool.tool}>
                      {(tool) => (
                        <div class="flex min-w-0 flex-col gap-tight pl-base">
                          <span class="font-mono text-note text-text-quiet">{tool().tool}</span>
                          <UsageUses
                            uses={tool().uses.map((one) => ({
                              run: one.run,
                              resident: one.resident ?? null,
                              at: one.at,
                              part: null,
                              outcome: one.outcome,
                            }))}
                            days={tool().per_day}
                          />
                        </div>
                      )}
                    </For>
                  </li>
                )}
              </For>
            </ul>
          )}
        </Match>
      </Switch>
    </section>
  );
}
