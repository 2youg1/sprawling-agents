// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tool servers a building may reach, one building at a time. Two
// doors in: a Composio server, spelled as the three things Composio
// hands out (server id, user id, api key - the key is enrolled into the
// vault and the header carries its reference, never the key), or a
// custom server over http or stdio. Removing is one click on the row.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import { configureMcp } from "../core/commands";
import { MAYOR, buildingOf } from "../core/route";
import { enrol } from "../core/enrol";
import type { Enrolment } from "../core/enrol";
import type { Address, McpServer } from "../wire";
import { ServerLabel } from "../wire";
import { useCommand, useSay, useUi } from "../ui";

const HALL = buildingOf(MAYOR);
const LABEL = /^[a-z][a-z0-9-]*$/;
const COMPOSIO_BASE = "https://backend.composio.dev/v3/mcp/";

type Door = "composio" | "http" | "stdio";

const field =
  "min-w-0 rounded-control bg-g2 px-base py-snug font-mono text-note text-text outline-none placeholder:text-text-disabled";

function ServerRow(props: { readonly server: McpServer; readonly onRemove: () => void }) {
  const say = useSay();
  const transport = () => props.server.transport;
  const line = () => {
    const t = transport();
    return "stdio" in t ? [t.stdio.command, ...t.stdio.args].join(" ") : t.http.url;
  };
  const composio = () => {
    const t = transport();
    return "http" in t && t.http.url.startsWith(COMPOSIO_BASE);
  };
  return (
    <li class="flex items-center gap-base rounded-card bg-g1 px-base py-snug text-note">
      <span class="w-figure shrink-0 truncate font-label text-text">{props.server.label}</span>
      <span class="shrink-0 rounded-pill bg-g2 px-snug text-text-faint">
        {composio() ? say("mcp_composio") : "stdio" in transport() ? say("mcp_kind_stdio") : say("mcp_kind_http")}
      </span>
      <span class="flex-1 truncate font-mono text-text-faint" title={line()}>
        {line()}
      </span>
      <button
        type="button"
        class="rounded-control px-snug py-tight text-text-faint hover:bg-g2 hover:text-alert"
        onClick={() => {
          props.onRemove();
        }}
      >
        {say("setup_remove")}
      </button>
    </li>
  );
}

// The list and the add form for one building; the welcome page mounts
// it for the hall alone.
export function McpForm(props: { readonly addr: Address }) {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [door, setDoor] = createSignal<Door>("composio");
  const [label, setLabel] = createSignal("");
  const [serverId, setServerId] = createSignal("");
  const [userId, setUserId] = createSignal("");
  const [apiKey, setApiKey] = createSignal("");
  const [url, setUrl] = createSignal("");
  const [header, setHeader] = createSignal("");
  const [line, setLine] = createSignal("");
  const [keeping, setKeeping] = createSignal<"idle" | "sending" | "refused">("idle");

  const building = createMemo(() => ui.conn.asking.ask({ building_view: { addr: props.addr } }));
  const servers = createMemo<readonly McpServer[]>(() => {
    const answer = building()();
    return answer !== undefined && "building" in answer ? answer.building.mcp : [];
  });
  const name = () => label().trim();
  const ready = () => {
    if (!LABEL.test(name()) || servers().some((s) => s.label === name())) return false;
    switch (door()) {
      case "composio":
        return serverId().trim() !== "";
      case "http":
        return url().trim() !== "";
      case "stdio":
        return line().trim() !== "";
    }
  };
  const put = (transport: McpServer["transport"]) => {
    if (command(configureMcp(props.addr, [...servers(), { label: ServerLabel.make(name()), transport }]))) {
      setLabel("");
      setServerId("");
      setUserId("");
      setApiKey("");
      setUrl("");
      setHeader("");
      setLine("");
      setKeeping("idle");
    }
  };
  const add = () => {
    if (!ready()) return;
    switch (door()) {
      case "http":
        put({ http: { url: url().trim(), header: header().trim() === "" ? null : header().trim() } });
        return;
      case "stdio": {
        const [program, ...args] = line().trim().split(/\s+/);
        put({ stdio: { command: program ?? "", args } });
        return;
      }
      case "composio": {
        const user = userId().trim();
        const staticTarget = `${COMPOSIO_BASE}${serverId().trim()}${user === "" ? "" : `?user_id=${encodeURIComponent(user)}`}`;
        const key = apiKey().trim();
        if (key === "") {
          put({ http: { url: staticTarget, header: null } });
          return;
        }
        setKeeping("sending");
        // The label as it stands when the key leaves; the transport is
        // built from that, not from whatever the fields say later.
        const staticLabel = name();
        const settle = (outcome: Enrolment) => {
          if (outcome.kind === "stored") {
            put({ http: { url: staticTarget, header: `x-api-key: ${outcome.reference}` } });
          } else {
            setKeeping("refused");
          }
        };
        void enrol(ui.origin, "mcp", `composio-${staticLabel}`, key).then(settle);
      }
    }
  };
  const doorClass = (which: Door) =>
    `rounded-pill px-base py-tight text-label ${door() === which ? "bg-g3 text-text" : "text-text-faint hover:text-text-quiet"}`;

  return (
    <div class="flex flex-col gap-base">
      <Show when={servers().length > 0} fallback={<p class="text-note text-text-disabled">{say("mcp_none")}</p>}>
        <ul class="flex flex-col gap-tight">
          <For each={servers()}>
            {(server) => (
              <ServerRow
                server={server}
                onRemove={() => command(configureMcp(props.addr, servers().filter((each) => each.label !== server.label)))}
              />
            )}
          </For>
        </ul>
      </Show>
      <div class="flex flex-col gap-snug rounded-panel bg-g1/60 p-base">
        <div class="flex flex-wrap items-center gap-snug">
          <button type="button" class={doorClass("composio")} onClick={() => setDoor("composio")}>
            {say("mcp_composio")}
          </button>
          <button type="button" class={doorClass("http")} onClick={() => setDoor("http")}>
            {say("mcp_kind_http")}
          </button>
          <button type="button" class={doorClass("stdio")} onClick={() => setDoor("stdio")}>
            {say("mcp_kind_stdio")}
          </button>
          <span class="flex-1" />
          <Show when={door() === "composio"}>
            <a href="https://platform.composio.dev" target="_blank" rel="noreferrer" class="text-note text-text-faint">
              {say("mcp_open_composio")} ↗
            </a>
          </Show>
        </div>
        <div class="grid gap-snug md:grid-cols-[1fr_2fr]">
          <input
            class={field}
            placeholder={say("mcp_label")}
            value={label()}
            onInput={(event) => setLabel(event.currentTarget.value)}
            aria-invalid={label() !== "" && !LABEL.test(name()) ? true : undefined}
          />
          <Switch>
            <Match when={door() === "composio"}>
              <input class={field} placeholder={say("mcp_server_id")} value={serverId()} onInput={(event) => setServerId(event.currentTarget.value)} />
              <input class={field} placeholder={say("mcp_user_id")} value={userId()} onInput={(event) => setUserId(event.currentTarget.value)} />
              <input
                class={field}
                type="password"
                placeholder={say("mcp_api_key")}
                value={apiKey()}
                onInput={(event) => setApiKey(event.currentTarget.value)}
              />
            </Match>
            <Match when={door() === "http"}>
              <input class={field} placeholder={say("mcp_url")} value={url()} onInput={(event) => setUrl(event.currentTarget.value)} />
              <span />
              <input class={field} placeholder={`${say("mcp_header")} · Name: value`} value={header()} onInput={(event) => setHeader(event.currentTarget.value)} />
            </Match>
            <Match when={door() === "stdio"}>
              <input class={field} placeholder={say("mcp_command")} value={line()} onInput={(event) => setLine(event.currentTarget.value)} />
            </Match>
          </Switch>
        </div>
        <div class="flex items-center gap-base">
          <button
            type="button"
            class="rounded-control bg-accent px-base py-tight text-label text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
            disabled={!ready() || keeping() === "sending"}
            onClick={add}
          >
            {say("mcp_add")}
          </button>
          <Show when={keeping() === "refused"}>
            <span class="text-note text-alert">{say("link_refused")}</span>
          </Show>
        </div>
      </div>
    </div>
  );
}

export function Mcp() {
  const ui = useUi();
  const say = useSay();
  const city = ui.conn.asking.ask("city_view");
  const buildings = createMemo<readonly Address[]>(() => {
    const answer = city();
    if (answer === undefined || !("city" in answer)) return [HALL];
    const all = answer.city.buildings.map((b) => b.addr);
    return [HALL, ...all.filter((addr) => addr !== HALL).sort((a, b) => a.localeCompare(b))];
  });
  const [chosen, setChosen] = createSignal<Address>(HALL);

  return (
    <div class="mx-auto flex w-full max-w-page flex-1 flex-col px-pane py-wide">
      <div class="mb-wide flex items-baseline gap-base">
        <h1 class="text-title font-title">{say("mcp_title")}</h1>
      </div>
      <div class="flex min-h-0 flex-1 gap-wide">
        <nav class="w-tree shrink-0" aria-label={say("mcp_building")}>
          <ul class="flex flex-col gap-tight">
            <For each={buildings()}>
              {(addr) => (
                <li>
                  <button
                    type="button"
                    class={`w-full rounded-control px-base py-tight text-left text-note ${chosen() === addr ? "bg-g2 text-text" : "text-text-quiet hover:bg-g1"}`}
                    onClick={() => setChosen(addr)}
                    aria-current={chosen() === addr ? "true" : undefined}
                  >
                    {addr === HALL ? say("city_hall") : addr}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </nav>
        <section class="min-w-0 flex-1">
          <McpForm addr={chosen()} />
        </section>
      </div>
    </div>
  );
}
