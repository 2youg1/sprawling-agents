// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// `claude mcp list`, as a page: one line per server with its transport,
// where it stands and how many tools it offers, and a row that opens on
// the tool list with each tool's input schema.
//
// A server nobody has reached yet says so rather than looking healthy.
// The three states come from the city's own handshake, so what a person
// reads here and what a model is given cannot disagree.

import { For, Show, createSignal } from "solid-js";

import type { McpServer, McpServerHealth, McpState } from "../../wire";
import { Badge } from "../parts/badge";
import { Button } from "../parts/button";
import { EmptyState } from "../parts/empty";
import { useSay } from "../../ui";

// The one place a transport becomes the line a person reads.
function targetOf(server: McpServer): string {
  const transport = server.transport;
  if ("stdio" in transport) return [transport.stdio.command, ...transport.stdio.args].join(" ");
  if ("http" in transport) return transport.http.url;
  return transport.sse.url;
}

function transportWord(server: McpServer): "mcp_kind_stdio" | "mcp_kind_http" | "mcp_kind_sse" {
  const transport = server.transport;
  if ("stdio" in transport) return "mcp_kind_stdio";
  if ("http" in transport) return "mcp_kind_http";
  return "mcp_kind_sse";
}

// The name/value rows a server is configured with, whichever of the two
// tables this transport keeps them in.
function pairsOf(server: McpServer): readonly (readonly [string, string])[] {
  const transport = server.transport;
  if ("stdio" in transport) return transport.stdio.env;
  if ("http" in transport) return transport.http.headers;
  return transport.sse.headers;
}

function pairCaption(server: McpServer): "mcp_env" | "mcp_header" {
  return "stdio" in server.transport ? "mcp_env" : "mcp_header";
}

export function Servers(props: {
  readonly servers: readonly McpServer[];
  readonly health: readonly McpServerHealth[];
  readonly onCheck: () => void;
  readonly onRemove: (label: string) => void;
}) {
  const say = useSay();
  const [open, setOpen] = createSignal<string | null>(null);
  const stateOf = (label: string): McpState | undefined =>
    props.health.find((line) => line.label === label)?.state;

  return (
    <Show
      when={props.servers.length > 0}
      fallback={<EmptyState text={say("mcp_none")} />}
    >
      <div class="flex flex-col gap-tight">
        <div class="flex justify-end">
          <Button label={say("mcp_check")} tone="quiet" onPress={props.onCheck} />
        </div>
        <ul class="flex flex-col gap-tight" aria-label={say("mcp_servers")}>
          <For each={props.servers}>
            {(server) => (
              <li class="rounded-card bg-g1">
                <div class="flex min-w-0 items-center gap-base px-base py-snug text-note">
                  <span class="w-figure shrink-0 truncate font-label text-text">{server.label}</span>
                  <Badge text={say(transportWord(server))} />
                  <StateBadge state={stateOf(server.label)} />
                  <span class="flex items-center gap-tight text-text-faint">
                    {say("mcp_tools")}
                    <span class="text-text-disabled">{toolCount(stateOf(server.label))}</span>
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
                    <Show when={pairsOf(server).length > 0}>
                      <div class="flex min-w-0 flex-col gap-tight">
                        <span class="text-text-quiet">{say(pairCaption(server))}</span>
                        <For each={pairsOf(server)}>
                          {(pair) => (
                            <span class="min-w-0 break-all font-mono text-text">
                              {pair[0]}
                              <span class="text-text-disabled"> = </span>
                              {pair[1]}
                            </span>
                          )}
                        </For>
                      </div>
                    </Show>
                    <Standing state={stateOf(server.label)} />
                  </div>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </div>
    </Show>
  );
}

// How many tools the server listed, or an em dash where nobody has
// asked: "none" and "not asked" are different facts about a server.
function toolCount(state: McpState | undefined): string {
  if (state === undefined) return "—";
  return "connected" in state ? String(state.connected.tools.length) : "—";
}

function StateBadge(props: { readonly state: McpState | undefined }) {
  const say = useSay();
  return (
    <Show when={props.state} fallback={<Badge text={say("mcp_state_unknown")} dot />}>
      {(state) => {
        const held = state();
        if ("connected" in held) return <Badge text={say("mcp_state_connected")} dot weight="live" />;
        if ("authenticating" in held) return <Badge text={say("mcp_state_auth")} dot weight="live" />;
        return <Badge text={say("mcp_state_failed")} dot weight="alert" />;
      }}
    </Show>
  );
}

// The three-part refusal, drawn verbatim: what failed, on what, and what
// a person can do about it. The city composed it; nothing here rewords
// it, because a second wording would be a second authority on why this
// server is not answering.
function Standing(props: { readonly state: McpState | undefined }) {
  const say = useSay();
  return (
    <Show when={props.state} fallback={<p class="text-text-faint">{say("mcp_state_unknown")}</p>}>
      {(state) => {
        const held = state();
        if ("authenticating" in held) {
          return <p class="text-text-faint">{held.authenticating.recovery}</p>;
        }
        if (!("connected" in held)) {
          const refusal = held.failed.refusal;
          return (
            <p class="text-text-faint">
              {refusal.action}: {refusal.subject} — {refusal.recovery}
            </p>
          );
        }
        return (
          <ul class="flex flex-col gap-tight">
            <For each={held.connected.tools}>
              {(tool) => (
                <li class="flex min-w-0 flex-col gap-tight">
                  <span class="font-mono text-text">{tool.name}</span>
                  <span class="text-text-faint">{tool.disclosure}</span>
                  <pre class="min-w-0 overflow-x-auto rounded-control bg-g2 px-base py-snug font-mono text-text-quiet">
                    {JSON.stringify(tool.input_schema, null, 2)}
                  </pre>
                </li>
              )}
            </For>
          </ul>
        );
      }}
    </Show>
  );
}
