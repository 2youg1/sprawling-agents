// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Settings: eight groups, one screen each, and beside each screen the
// city's own `config.toml` - so somebody who knows the file can check
// the page against what they already know.
//
// **The fragment is quoted from the file or it is not drawn.** This
// column used to build the text itself, and what it built was a
// grammar the city's own reader refuses: `[model_providers.<name>]`
// is Codex's spelling, and a person who copied it into their
// `CONFIG.toml` got a refusal naming three sections none of which
// they had written. Reading the file is `Query::Config` (roadmap
// 3.3), which this build cannot ask, so the column says that and
// shows nothing.
//
// The groups are a left column at 1024 px and wider, and a bottom bar
// below that; the `config.toml` column joins them at 1440 px and
// wider. The screen is a signal rather than a route because
// `core/route.ts` is the address bar's one authority and a group is
// not a page a person bookmarks.
//
// A group whose section another view owns mounts that view. There is
// no MCP group: it was a link to `#/mcp`, which the rail already
// reaches, so the settings page carried a way in that did nothing but
// stand between a person and the page.
//
// The page is centred and capped at the page width, so a wide window
// leaves margins rather than a column of text against the left edge,
// and the reading column is held to a measure only once the window is
// wide enough to spare it.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";
import type { JSX } from "solid-js";

import { setAutonomy } from "../core/commands";
import { LANGS, endonym } from "../core/lang";
import type { Key } from "../core/lang";
import { QUERIES } from "../core/asking";
import { preferences } from "../core/prefs";
import { toFragment } from "../core/route";
import type { Autonomy, EndpointsAnswer } from "../wire";
import { ResidentId } from "../wire";
import { useCommand, useLanguage, useSay, useUi } from "../ui";
import { Machine } from "./machine";
import { EmptyState } from "./parts/empty";
import { Segmented } from "./parts/segmented";
import { EffortSection } from "./shared/effort";
import { ProviderDoor } from "./shared/provider";
import { KeysSection } from "./setup/keys";
import { AppearanceSection } from "./setup/appearance";
import { ModelChoice } from "./setup/models";
import { SkillsSection } from "./setup/skills";
import { PROXYINGS, proxyingNote } from "./setup/providers";
import { Kept } from "./setup/kept";

// The eight screens, in the order a city is set up.
const GROUPS = [
  "accounts",
  "run",
  "network",
  "tools",
  "skills",
  "appearance",
  "keys",
  "advanced",
] as const;
type Group = (typeof GROUPS)[number];

// The three groups whose answers `core/prefs.ts` keeps, and therefore
// the three whose heading says where those answers live. They are the
// three arms of the switch below that mount a screen reading the
// preference door; the other five collect the city's own record - an
// endpoint, a model, an autonomy - about which a badge naming a
// keeper would say nothing true.
const PREFERRED: readonly Group[] = ["network", "appearance", "keys"];

// Who answers an approval, as the three settings a person picks
// between. `delegate` carries an address on the wire, and the clerk is
// the only resident this screen delegates to.
type AutonomySetting = "owner" | "delegate";

const AUTONOMIES = [
  ["owner", "autonomy_owner"],
  ["delegate", "autonomy_clerk"],
] as const satisfies readonly (readonly [AutonomySetting, Key])[];

function autonomySetting(held: Autonomy): AutonomySetting {
  return typeof held === "string" ? held : "delegate";
}

function autonomyOf(setting: AutonomySetting): Autonomy {
  switch (setting) {
    case "owner":
      return "owner";
    case "delegate":
      return { delegate: ResidentId.make("hall/clerk") };
  }
}

// The column that quotes the city's own `config.toml`, standing empty
// until something can read the file. One place, so the day
// `Query::Config` answers there is one column to fill and no second
// spelling of the file to retire first.
function Toml() {
  const say = useSay();
  return (
    <aside class="flex min-w-0 flex-col gap-snug" aria-label={say("setup_toml")}>
      <h2 class="text-label font-label text-text-quiet">{say("setup_toml")}</h2>
      <EmptyState text={say("setup_toml_unread")} />
    </aside>
  );
}

// How a call leaves this machine: the proxy rule a provider attached
// from now on starts with.
//
// **The rule belongs to the endpoint, not to the city**, because a
// relay that must see every request and a local inference server that
// a relay would break can sit on one machine at once. So this screen
// settles the starting point, the provider form carries it per
// endpoint, and an endpoint already attached keeps the rule the city
// recorded for it.
//
function Network() {
  const say = useSay();
  const kept = preferences();
  return (
    <div class="flex flex-col gap-snug text-note text-text-quiet">
      <span>{say("setup_network_default")}</span>
      <Segmented
        label={say("setup_network_default")}
        options={PROXYINGS.map(([setting, word]) => ({ value: setting, label: say(word) }))}
        held={kept.held().proxying}
        onPick={kept.setProxying}
      />
      <span class="text-text-faint">{say("setup_proxying_help")}</span>
      <Show when={proxyingNote(kept.held().proxying)}>
        {(note) => <span class="text-text-faint">{say(note())}</span>}
      </Show>
      <span class="text-text-faint">{say("setup_network_new_only")}</span>
    </div>
  );
}

export function Setup() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const setLanguage = useLanguage();
  const [group, setGroup] = createSignal<Group>("accounts");
  const endpoints = ui.conn.asking.ask(QUERIES.endpoints);
  const answer = createMemo<EndpointsAnswer | undefined>(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });
  const governance = ui.conn.asking.ask(QUERIES.governance);
  const autonomy = createMemo<Autonomy | undefined>(() => {
    const held = governance();
    return held !== undefined && "governance" in held ? held.governance.autonomy : undefined;
  });
  const tab = (each: Group): JSX.Element => (
    <button
      type="button"
      class={`flex h-bar shrink-0 items-center rounded-control px-base text-label lg:h-auto lg:py-snug ${
        group() === each ? "bg-g2 text-text" : "text-text-faint hover:bg-g1 hover:text-text-quiet"
      }`}
      aria-current={group() === each ? "page" : undefined}
      onClick={() => setGroup(each)}
    >
      {say(`setup_group_${each}`)}
    </button>
  );

  return (
    <div class="mx-auto flex min-h-0 w-full max-w-page flex-1 flex-col-reverse gap-wide px-pane py-wide lg:flex-row">
      <nav
        class="sticky bottom-0 z-10 -mx-pane flex h-bar shrink-0 items-center gap-tight overflow-x-auto border-t border-g1 bg-g0 px-pane lg:static lg:mx-0 lg:h-auto lg:w-tree lg:flex-col lg:items-stretch lg:overflow-visible lg:border-0 lg:px-0"
        aria-label={say("setup_groups")}
      >
        <For each={GROUPS}>{(each) => tab(each)}</For>
      </nav>

      <div class="flex min-w-0 flex-1 flex-col gap-wide wide:flex-row wide:items-start">
        <div class="flex min-w-0 flex-1 flex-col gap-base">
          <p class="text-note text-text-faint">{say("nav_settings")}</p>
          <div class="flex flex-wrap items-baseline gap-base">
            <h1 class="text-title font-title">{say(`setup_group_${group()}`)}</h1>
            <Show when={PREFERRED.includes(group())}>
              <Kept keeper={ui.prefs.keeper()} />
            </Show>
          </div>
          <div class={group() === "tools" ? "min-w-0" : "min-w-0 wide:max-w-measure"}>
            <Switch>
              <Match when={group() === "accounts"}>
                <div class="flex flex-col gap-base">
                  <ProviderDoor />
                  <h2 class="text-label font-label text-text-quiet">{say("setup_models")}</h2>
                  <Show when={answer()}>{(held) => <ModelChoice answer={held()} />}</Show>
                </div>
              </Match>
              <Match when={group() === "run"}>
                <div class="flex flex-col gap-wide">
                  <EffortSection />
                  <div class="flex flex-col gap-tight text-note text-text-quiet">
                    {say("setup_autonomy")}
                    <Show when={autonomy()}>
                      {(held) => (
                        <Segmented
                          label={say("setup_autonomy")}
                          options={AUTONOMIES.map(([setting, word]) => ({
                            value: setting,
                            label: say(word),
                          }))}
                          held={autonomySetting(held())}
                          onPick={(setting) => {
                            command(setAutonomy("city", autonomyOf(setting)));
                          }}
                        />
                      )}
                    </Show>
                  </div>
                </div>
              </Match>
              <Match when={group() === "network"}>
                <Network />
              </Match>
              <Match when={group() === "tools"}>
                <Machine />
              </Match>
              <Match when={group() === "skills"}>
                <SkillsSection />
              </Match>
              <Match when={group() === "appearance"}>
                <AppearanceSection />
                <div class="flex flex-col gap-tight text-note text-text-quiet">
                  {say("setup_language")}
                  <Segmented
                    label={say("setup_language")}
                    options={LANGS.map((lang) => ({ value: lang, label: endonym(lang) }))}
                    held={ui.prefs.held().lang}
                    onPick={setLanguage}
                  />
                </div>
              </Match>
              <Match when={group() === "keys"}>
                <KeysSection />
              </Match>
              <Match when={group() === "advanced"}>
                <a
                  href={toFragment({ kind: "welcome" })}
                  class="inline-block rounded-control bg-g2 px-base py-snug text-label hover:bg-g3"
                >
                  {say("setup_rerun")}
                </a>
              </Match>
            </Switch>
          </div>
        </div>

        <div class="min-w-0 wide:w-tree wide:shrink-0">
          <Toml />
        </div>
      </div>
    </div>
  );
}
