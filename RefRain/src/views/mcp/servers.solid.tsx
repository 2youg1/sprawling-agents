// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Match, Show, Switch, createSignal } from "solid-js";
import { say, type Lang } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { McpServer, McpServerHealth, McpState } from "../../wire";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import Dialog from "../parts/dialog_view";
import EmptyState from "../parts/empty.solid";
import { recoveryWords } from "../parts/notice_title";
import Row, { RowList } from "../parts/row.solid";

function configuration(server: McpServer) {
  const transport = server.transport;
  if ("stdio" in transport)
    return {
      kind: "mcp_kind_stdio",
      word: "mcp_env",
      target: [transport.stdio.command, ...transport.stdio.args].join(" "),
      pairs: transport.stdio.env,
    } as const;
  if ("http" in transport)
    return {
      kind: "mcp_kind_http",
      word: "mcp_header",
      target: transport.http.url,
      pairs: transport.http.headers,
    } as const;
  return {
    kind: "mcp_kind_sse",
    word: "mcp_header",
    target: transport.sse.url,
    pairs: transport.sse.headers,
  } as const;
}

function standing(state: McpState | undefined) {
  if (state === undefined)
    return { label: "mcp_state_unknown", weight: undefined, count: "—" } as const;
  if ("connected" in state)
    return {
      label: "mcp_state_connected",
      weight: "live",
      count: String(state.connected.tools.length),
    } as const;
  if ("authenticating" in state)
    return { label: "mcp_state_auth", weight: "live", count: "—" } as const;
  return { label: "mcp_state_failed", weight: "alert", count: "—" } as const;
}

function StatusDetails(props: { readonly state: McpState | undefined; readonly lang: Lang }) {
  const failed = () => {
    const value = props.state;
    return value && "failed" in value ? value.failed.refusal : undefined;
  };
  const auth = () => {
    const value = props.state;
    return value && "authenticating" in value ? value.authenticating : undefined;
  };
  const tools = () => {
    const value = props.state;
    return value && "connected" in value ? value.connected.tools : undefined;
  };
  return (
    <Switch>
      <Match when={props.state === undefined}>
        <p class="text-text-faint">{say(props.lang, "mcp_state_unknown")}</p>
      </Match>
      <Match when={auth()}>{(value) => <p class="text-text-faint">{value().recovery}</p>}</Match>
      <Match when={failed()}>
        {(error) => (
          <p class="text-text-faint">
            {error().action}: {error().subject} —{" "}
            {recoveryWords(props.lang, error().code, error().recovery)}
          </p>
        )}
      </Match>
      <Match when={tools()}>
        {(rows) => (
          <ul class="flex min-w-0 flex-col gap-tight">
            <For each={rows()} keyed={(tool) => tool.name}>
              {(tool) => (
                <li class="flex min-w-0 flex-col gap-tight">
                  <span class="font-mono text-text">{tool().name}</span>
                  <span class="text-text-faint">{tool().disclosure}</span>
                  <pre class="min-w-0 overflow-x-auto rounded-control bg-raised px-base py-snug font-mono text-text-quiet">
                    {JSON.stringify(tool().input_schema, null, 2)}
                  </pre>
                </li>
              )}
            </For>
          </ul>
        )}
      </Match>
    </Switch>
  );
}

export default function Servers(props: {
  readonly servers: readonly McpServer[];
  readonly health: readonly McpServerHealth[];
  readonly onCheck: () => void;
  readonly onRemove: (label: string) => void;
}) {
  const lang = storeValue(ui().lang);
  const [expanded, setExpanded] = createSignal<string | null>(null);
  const [confirming, setConfirming] = createSignal<string | null>(null);
  return (
    <>
      <Show
        when={props.servers.length > 0}
        fallback={<EmptyState missing="mcp_none" seat="region" />}
      >
        <div class="flex flex-col gap-tight">
          <div class="flex justify-end">
            <Button label={say(lang(), "mcp_check")} tone="quiet" onPress={props.onCheck} />
          </div>
          <RowList label={say(lang(), "mcp_servers")}>
            <For each={props.servers} keyed={(server) => server.label}>
              {(server) => {
                const config = () => configuration(server());
                const state = () =>
                  props.health.find((line) => line.label === server().label)?.state;
                const status = () => standing(state());
                return (
                  <>
                    <Row
                      primary={server().label}
                      secondary={config().target}
                      status={
                        <span class="flex items-center gap-tight text-text-faint">
                          <Badge text={say(lang(), config().kind)} />
                          <Badge
                            text={say(lang(), status().label)}
                            dot
                            weight={status().weight ?? "quiet"}
                          />
                          {say(lang(), "mcp_tools")}
                          <span class="text-text-faint">{status().count}</span>
                        </span>
                      }
                      actions={
                        <>
                          <Button
                            label={say(lang(), "mcp_expand")}
                            tone="quiet"
                            onPress={() => {
                              const label = server().label;
                              setExpanded((old) => (old === label ? null : label));
                            }}
                          />
                          <Button
                            label={say(lang(), "setup_remove")}
                            tone="destructive"
                            onPress={() => {
                              setConfirming(server().label);
                            }}
                          />
                        </>
                      }
                    />
                    <Show when={expanded() === server().label}>
                      <li class="flex flex-col gap-tight border-b border-edge px-base py-snug text-note">
                        <div class="flex min-w-0 items-baseline gap-snug">
                          <span class="shrink-0 text-text-quiet">{say(lang(), "mcp_target")}</span>
                          <span class="min-w-0 wrap-anywhere font-mono text-text">
                            {config().target}
                          </span>
                        </div>
                        <Show when={config().pairs.length > 0}>
                          <div class="flex min-w-0 flex-col gap-tight">
                            <span class="text-text-quiet">{say(lang(), config().word)}</span>
                            <For each={config().pairs}>
                              {(pair) => (
                                <span class="min-w-0 wrap-anywhere font-mono text-text">
                                  {pair[0]}
                                  <span class="text-text-faint">{" = "}</span>
                                  {pair[1]}
                                </span>
                              )}
                            </For>
                          </div>
                        </Show>
                        <StatusDetails lang={lang()} state={state()} />
                      </li>
                    </Show>
                  </>
                );
              }}
            </For>
          </RowList>
        </div>
      </Show>
      <Dialog
        open={confirming() !== null}
        title={say(lang(), "part_remove_server")}
        detail={say(lang(), "part_remove_server_detail")}
        confirmLabel={say(lang(), "setup_remove")}
        cancelLabel={say(lang(), "part_cancel")}
        destructive
        onConfirm={() => {
          const label = confirming();
          setConfirming(null);
          if (label !== null) props.onRemove(label);
        }}
        onCancel={() => {
          setConfirming(null);
        }}
      />
    </>
  );
}
