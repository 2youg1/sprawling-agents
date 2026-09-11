// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `claude mcp list`, as a page: one line per server with its transport,
// where it stands and how many tools it offers, and a row that opens on
// the tool list.
//
// The state and the tool count are drawn as unchecked rather than left
// out. A server this city has never spoken to looks exactly like one it
// spoke to successfully, and the row says which of those two it is
// showing - today, always the first.

import { For, Show, createSignal } from "solid-js";

import type { McpServer } from "../../wire";
import { Badge } from "../parts/badge";
import { Button } from "../parts/button";
import { EmptyState } from "../parts/empty";
import { useSay } from "../../ui";

// The one place a transport becomes the line a person reads.
function targetOf(server: McpServer): string {
  const transport = server.transport;
  return "stdio" in transport
    ? [transport.stdio.command, ...transport.stdio.args].join(" ")
    : transport.http.url;
}

function transportWord(server: McpServer): "mcp_kind_stdio" | "mcp_kind_http" {
  return "stdio" in server.transport ? "mcp_kind_stdio" : "mcp_kind_http";
}

function headerOf(server: McpServer): string | null {
  const transport = server.transport;
  if ("stdio" in transport) return null;
  return transport.http.header ?? null;
}

export function Servers(props: {
  readonly servers: readonly McpServer[];
  readonly onRemove: (label: string) => void;
}) {
  const say = useSay();
  const [open, setOpen] = createSignal<string | null>(null);

  return (
    <Show
      when={props.servers.length > 0}
      fallback={<EmptyState text={say("mcp_none")} />}
    >
      <ul class="flex flex-col gap-tight" aria-label={say("mcp_servers")}>
        <For each={props.servers}>
          {(server) => (
            <li class="rounded-card bg-g1">
              <div class="flex min-w-0 items-center gap-base px-base py-snug text-note">
                <span class="w-figure shrink-0 truncate font-label text-text">{server.label}</span>
                <Badge text={say(transportWord(server))} />
                <Badge text={say("mcp_state_unknown")} dot />
                <span class="flex items-center gap-tight text-text-faint">
                  {say("mcp_tools")}
                  <span class="text-text-disabled">—</span>
                </span>
                <span class="min-w-0 flex-1 truncate font-mono text-text-faint" title={targetOf(server)}>
                  {targetOf(server)}
                </span>
                <Button
                  label={say("mcp_expand")}
                  tone="quiet"
                  onPress={() => {
                    setOpen(open() === server.label ? null : server.label);
                  }}
                />
                <Button
                  label={say("setup_remove")}
                  tone="destructive"
                  onPress={() => {
                    props.onRemove(server.label);
                  }}
                />
              </div>
              <Show when={open() === server.label}>
                <div class="flex flex-col gap-tight border-t border-g2 px-base py-snug text-note">
                  <div class="flex min-w-0 items-baseline gap-snug">
                    <span class="shrink-0 text-text-quiet">{say("mcp_target")}</span>
                    <span class="min-w-0 break-all font-mono text-text">{targetOf(server)}</span>
                  </div>
                  <Show when={headerOf(server)}>
                    {(header) => (
                      <div class="flex min-w-0 items-baseline gap-snug">
                        <span class="shrink-0 text-text-quiet">{say("mcp_header")}</span>
                        <span class="min-w-0 break-all font-mono text-text">{header()}</span>
                      </div>
                    )}
                  </Show>
                  <p class="text-text-faint">{say("mcp_gap_health")}</p>
                </div>
              </Show>
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}
