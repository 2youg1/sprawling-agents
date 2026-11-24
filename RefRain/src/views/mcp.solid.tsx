// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show, createEffect, createMemo, createSignal } from "solid-js";
import { say, type Key } from "../core/lang";
import { storeValue } from "../core/solid_store";
import { ui } from "../ui";
import type { Address } from "../wire";
import DesktopForm from "./desktop.solid";
import ByCommand from "./mcp/by_command.solid";
import ByUrl from "./mcp/by_url.solid";
import Composio from "./mcp/composio.solid";
import ByJson from "./mcp/json.solid";
import { reachOf } from "./mcp/reach";
import Servers from "./mcp/servers.solid";
import Shells from "./mcp/shells.solid";
import McpUsage from "./mcp/usage.solid";
import Banner from "./parts/banner.solid";
import Page from "./parts/page.solid";
import Segmented from "./parts/segmented.solid";
import Tabs from "./parts/tabs.solid";
import { HALL, useBuildings } from "./shared/buildings";
import BuildingColumn from "./shared/buildings.solid";

const DOORS = ["command", "url", "json"] as const;
const SCOPES = ["city", "building", "machine"] as const;
type Door = (typeof DOORS)[number];
type Scope = (typeof SCOPES)[number];
const DOOR_WORD: Record<Door, Key> = {
  command: "mcp_door_command",
  url: "mcp_door_url",
  json: "mcp_door_json",
};
const SCOPE_WORD: Record<Scope, Key> = {
  city: "mcp_scope_city",
  building: "mcp_scope_building",
  machine: "mcp_scope_machine",
};

export default function Mcp(props: { readonly rank?: "page" | "section" | undefined }) {
  const lang = storeValue(ui().lang);
  const buildings = storeValue(useBuildings());
  const [chosen, setChosen] = createSignal<Address>(HALL);
  const [scope, setScope] = createSignal<Scope>("building");
  const [door, setDoor] = createSignal<Door>("command");
  const reason = createMemo(() => {
    const place = scope();
    return place === "building"
      ? null
      : say(lang(), place === "city" ? "mcp_gap_scope_city" : "mcp_gap_scope_machine");
  });
  const reach = reachOf({
    get addr() {
      return chosen();
    },
    get why() {
      return reason();
    },
  });
  const state = storeValue(reach.state);
  const intake = { ...reach.intake, taken: () => state().servers.map((server) => server.label) };
  createEffect(chosen, reach.watch);
  const lenses = createMemo(() => DOORS.map((id) => ({ id, label: say(lang(), DOOR_WORD[id]) })));
  const scopes = createMemo(() =>
    SCOPES.map((value) => ({ value, label: say(lang(), SCOPE_WORD[value]) })),
  );

  return (
    <Page
      title={say(lang(), "mcp_title")}
      rank={props.rank}
      aside={
        <>
          <span class="text-note text-text-quiet">{say(lang(), "mcp_scope")}</span>
          <Segmented
            label={say(lang(), "mcp_scope")}
            options={scopes()}
            held={scope()}
            onPick={setScope}
          />
        </>
      }
    >
      <Show when={reason()}>{(text) => <Banner text={text()} />}</Show>
      <div class="@container flex min-h-0 min-w-0 flex-1">
        <div class="flex min-h-0 min-w-0 flex-1 flex-col gap-wide @min-[64rem]:silver-columns @min-[64rem]:items-start">
          <BuildingColumn
            label={say(lang(), "mcp_building")}
            buildings={buildings()}
            chosen={chosen()}
            hall={say(lang(), "city_hall")}
            onPick={setChosen}
          />
          <section class="flex min-w-0 flex-col gap-wide" aria-label={say(lang(), "mcp_doors")}>
            <Servers
              servers={state().servers}
              health={state().health}
              onCheck={reach.check}
              onRemove={reach.withdraw}
            />
            <div class="flex flex-col gap-base border-t border-edge pt-base">
              <h2 class="text-note text-text-faint">{say(lang(), "mcp_doors")}</h2>
              <Tabs
                label={say(lang(), "mcp_doors")}
                lenses={lenses()}
                current={door()}
                onPick={(id) => {
                  const found = DOORS.find((candidate) => candidate === id);
                  if (found !== undefined) setDoor(found);
                }}
                panel={(lens) => (
                  <>
                    <Show when={lens.id === "command"}>
                      <ByCommand intake={intake} />
                    </Show>
                    <Show when={lens.id === "url"}>
                      <ByUrl
                        intake={intake}
                        onCommandDoor={() => {
                          setDoor("command");
                        }}
                      />
                    </Show>
                    <Show when={lens.id === "json"}>
                      <ByJson intake={intake} />
                    </Show>
                  </>
                )}
              />
            </div>
          </section>
          <div class="flex min-w-0 flex-col gap-wide">
            <section class="flex flex-col gap-base border-t border-edge pt-base">
              <h2 class="text-note text-text-faint">{say(lang(), "mcp_composio")}</h2>
              <Composio intake={intake} />
            </section>
            <section class="flex flex-col gap-base border-t border-edge pt-base">
              <h2 class="text-note text-text-faint">{say(lang(), "desktop_title")}</h2>
              <DesktopForm addr={chosen()} />
            </section>
            <div class="border-t border-edge pt-base">
              <McpUsage />
            </div>
            <div class="border-t border-edge pt-base">
              <Shells />
            </div>
          </div>
        </div>
      </div>
    </Page>
  );
}
