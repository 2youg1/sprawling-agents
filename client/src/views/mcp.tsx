// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The tool servers a scope may reach, laid out the way Claude Code
// taught people to expect: what is already reachable, three ways to add
// one more, and Composio as a directory.
//
// This file is the router. It owns the two choices that are the page's
// own - which scope a server is written into and which building's list
// is on screen - and hands every door one `Intake`, so a door never
// learns what a scope is and the scope rule stays in one place.

import { For, Show, createMemo, createSignal } from "solid-js";

import { Banner } from "./parts/banner";
import { ByCommand } from "./mcp/by_command";
import { ByJson } from "./mcp/by_json";
import { ByUrl } from "./mcp/by_url";
import { Composio } from "./mcp/composio";
import { DesktopForm } from "./desktop";
import { MAYOR, buildingOf } from "../core/route";
import { Servers } from "./mcp/servers";
import { Tabs } from "./parts/tabs";
import type { Address } from "../wire";
import { reachOf } from "./mcp/reach";
import { useSay, useUi } from "../ui";

const HALL = buildingOf(MAYOR);

type Door = "command" | "url" | "json";

// The three scopes Claude Code offers. Only the middle one has a field
// on the wire; the page says what the other two are waiting for rather
// than hiding them, because a person who used `--scope user` looks for
// them first.
type Scope = "city" | "building" | "machine";

const SCOPES: readonly Scope[] = ["city", "building", "machine"];

// The list and the three doors for one building. The welcome page
// mounts it for the hall alone, where there is no scope to choose and
// no Composio key to enrol yet.
export function McpForm(props: {
  readonly addr: Address;
  // The sentence that stops every send, when the chosen scope has no
  // field on the wire.
  readonly why?: () => string | null;
}) {
  const say = useSay();
  const [door, setDoor] = createSignal<Door>("command");
  // The lens id comes back as a string; this is where it becomes one of
  // the three doors again.
  const doorOf = (id: string): Door => (id === "url" || id === "json" ? id : "command");
  const reach = reachOf({
    get addr() {
      return props.addr;
    },
    get why() {
      return props.why?.() ?? null;
    },
  });

  return (
    <div class="flex flex-col gap-wide">
      <Servers servers={reach.servers()} onRemove={reach.withdraw} />
      <div class="flex flex-col gap-base rounded-panel bg-g1/60 p-base">
        <Tabs
          label={say("mcp_doors")}
          current={door()}
          onPick={(id) => {
            setDoor(doorOf(id));
          }}
          lenses={[
            { id: "command", label: say("mcp_door_command") },
            { id: "url", label: say("mcp_door_url") },
            { id: "json", label: say("mcp_door_json") },
          ]}
        />
        <Show when={door() === "command"}>
          <ByCommand intake={reach.intake} />
        </Show>
        <Show when={door() === "url"}>
          <ByUrl
            intake={reach.intake}
            onCommandDoor={() => {
              setDoor("command");
            }}
          />
        </Show>
        <Show when={door() === "json"}>
          <ByJson intake={reach.intake} />
        </Show>
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
  const [scope, setScope] = createSignal<Scope>("building");

  const scoped = (which: Scope) => {
    switch (which) {
      case "city":
        return say("mcp_scope_city");
      case "building":
        return say("mcp_scope_building");
      case "machine":
        return say("mcp_scope_machine");
    }
  };
  const why = (): string | null => {
    switch (scope()) {
      case "city":
        return say("mcp_gap_scope_city");
      case "building":
        return null;
      case "machine":
        return say("mcp_gap_scope_machine");
    }
  };
  const composio = reachOf({
    get addr() {
      return chosen();
    },
    get why() {
      return why();
    },
  });

  return (
    <div class="mx-auto flex w-full max-w-page flex-1 flex-col px-pane py-wide">
      <div class="mb-wide flex flex-wrap items-baseline gap-base">
        <h1 class="text-title font-title">{say("mcp_title")}</h1>
        <span class="text-note text-text-quiet">{say("mcp_scope")}</span>
        <For each={SCOPES}>
          {(which) => (
            <button
              type="button"
              aria-pressed={scope() === which}
              class={`rounded-pill px-base py-tight text-label ${
                scope() === which ? "bg-g3 text-text" : "text-text-faint hover:text-text-quiet"
              }`}
              onClick={() => {
                setScope(which);
              }}
            >
              {scoped(which)}
            </button>
          )}
        </For>
      </div>
      <Show when={why()}>{(said) => <Banner text={said()} weight="notice" />}</Show>
      <div class="flex min-h-0 flex-1 gap-wide">
        <nav class="w-tree shrink-0" aria-label={say("mcp_building")}>
          <ul class="flex flex-col gap-tight">
            <For each={buildings()}>
              {(addr) => (
                <li>
                  <button
                    type="button"
                    class={`w-full rounded-control px-base py-tight text-left text-note ${
                      chosen() === addr ? "bg-g2 text-text" : "text-text-quiet hover:bg-g1"
                    }`}
                    onClick={() => {
                      setChosen(addr);
                    }}
                    aria-current={chosen() === addr ? "true" : undefined}
                  >
                    {addr === HALL ? say("city_hall") : addr}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </nav>
        <section class="flex min-w-0 flex-1 flex-col gap-wide">
          <McpForm addr={chosen()} why={why} />
          <div>
            <h2 class="mb-base text-label font-label text-text-quiet">{say("mcp_composio")}</h2>
            <Composio intake={composio.intake} />
          </div>
          <div>
            <h2 class="mb-base text-label font-label text-text-quiet">{say("desktop_title")}</h2>
            <DesktopForm addr={chosen()} />
          </div>
        </section>
      </div>
    </div>
  );
}
