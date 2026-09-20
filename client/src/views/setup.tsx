// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Settings: nine groups, one screen each, and beside each screen the
// same settings as `config.toml` would spell them - so somebody who
// knows the file can check the page against what they already know.
//
// The groups are a left column at 1024 px and wider, and a bottom bar
// below that; the `config.toml` fragment joins them as a third column
// at 1440 px and wider. The screen is a signal rather than a route
// because `core/route.ts` is the address bar's one authority and a
// group is not a page a person bookmarks.
//
// A group whose section another view owns mounts that view.
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
import { browserRows, defaultProxying, setDefaultProxying } from "../core/prefs";
import { toFragment } from "../core/route";
import type { Autonomy, Effort, EndpointsAnswer, Proxying } from "../wire";
import { ResidentId } from "../wire";
import { useCommand, useSay, useUi } from "../ui";
import { Machine } from "./machine";
import { Button } from "./parts/button";
import { Segmented } from "./parts/segmented";
import { EffortSection } from "./shared/effort";
import { ProviderDoor } from "./shared/provider";
import { KeysSection } from "./setup/keys";
import { AppearanceSection } from "./setup/appearance";
import { ModelChoice } from "./setup/models";
import { PROXYINGS, proxyingNote } from "./setup/providers";

// The nine screens, in the order a city is set up.
const GROUPS = [
  "accounts",
  "run",
  "network",
  "tools",
  "mcp",
  "skills",
  "appearance",
  "keys",
  "advanced",
] as const;
type Group = (typeof GROUPS)[number];

// `wire_api` as Codex spells it, from the dialect the wire carries.
function wireApi(dialect: "anthropic" | "open_ai"): string {
  return dialect === "anthropic" ? "messages" : "chat";
}

// What the providers and the chosen models look like in `config.toml`.
function providersToml(answer: EndpointsAnswer | undefined): string | null {
  if (answer === undefined || answer.endpoints.length === 0) {
    return null;
  }
  const blocks = answer.endpoints.map((endpoint) =>
    [
      `[model_providers.${endpoint.name}]`,
      `base_url = "${endpoint.base_url}"`,
      `wire_api = "${wireApi(endpoint.dialect)}"`,
      endpoint.has_credential ? `env_key = "secret:providers/${endpoint.name}"` : "",
    ]
      .filter((line) => line !== "")
      .join("\n"),
  );
  const chosen = answer.chosen.map((each) => `${each.tag} = "${each.endpoint}/${each.model}"`);
  return chosen.length === 0
    ? blocks.join("\n\n")
    : [...blocks, ["[model]", ...chosen].join("\n")].join("\n\n");
}

// How hard the city thinks by default, in the section the city reads
// it from: `[model] effort`, which is the only key `ConfigFile` takes
// under `[model]` (`crates/city/src/config_layers.rs`).
//
// A person who has chosen no effort has no line in the file, and so
// no section either: absence is what lets the provider choose, and a
// level written here would be a level nobody asked for.
//
// Who answers an approval is not in this file. `set_autonomy` is a
// command the city records in its own governance, and the reader that
// parses this file refuses a key it does not know - so a fragment
// naming `[run] approvals` would hand a person a file their city
// would reject.
function modelToml(effort: Effort | null): string | null {
  return effort === null ? null : `[model]\neffort = "${effort}"`;
}

// Who answers an approval, as the three settings a person picks
// between. `delegate` carries an address on the wire, and the clerk is
// the only resident this screen delegates to.
type AutonomySetting = "owner" | "delegate" | "deferred";

const AUTONOMIES = [
  ["owner", "autonomy_owner"],
  ["delegate", "autonomy_clerk"],
  ["deferred", "autonomy_deferred"],
] as const satisfies readonly (readonly [AutonomySetting, Key])[];

function autonomySetting(held: Autonomy): AutonomySetting {
  return typeof held === "string" ? held : "delegate";
}

function autonomyOf(setting: AutonomySetting): Autonomy {
  switch (setting) {
    case "owner":
      return "owner";
    case "deferred":
      return "deferred";
    case "delegate":
      return { delegate: ResidentId.make("hall/clerk") };
  }
}

// The fragment beside a screen, when that screen's settings have a
// `config.toml` spelling today. A group whose settings the file does
// not carry yet shows none rather than a shape nobody has agreed to.
function Toml(props: { readonly text: string }) {
  const say = useSay();
  return (
    <aside class="flex min-w-0 flex-col gap-snug" aria-label={say("setup_toml")}>
      <div class="flex items-center gap-snug">
        <h2 class="text-label font-label text-text-quiet">{say("setup_toml")}</h2>
        <Button
          label={say("setup_copy")}
          tone="quiet"
          onPress={() => {
            void navigator.clipboard.writeText(props.text);
          }}
        />
      </div>
      <pre class="min-w-0 overflow-x-auto rounded-card bg-g1 p-base">
        <code class="whitespace-pre-wrap wrap-anywhere font-mono text-note text-text-quiet">{props.text}</code>
      </pre>
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
  const store = browserRows();
  const [rule, setRule] = createSignal<Proxying>(defaultProxying(store));
  return (
    <div class="flex flex-col gap-snug text-note text-text-quiet">
      <span>{say("setup_network_default")}</span>
      <Segmented
        label={say("setup_network_default")}
        options={PROXYINGS.map(([setting, word]) => ({ value: setting, label: say(word) }))}
        held={rule()}
        onPick={(next) => {
          setDefaultProxying(store, next);
          setRule(next);
        }}
      />
      <span class="text-text-faint">{say("setup_proxying_help")}</span>
      <Show when={proxyingNote(rule())}>
        {(note) => <span class="text-text-faint">{say(note())}</span>}
      </Show>
      <span class="text-text-faint">{say("setup_network_new_only")}</span>
    </div>
  );
}

export function SkillsNote() {
  const ui = useUi();
  const say = useSay();
  const path = () => `${ui.conn.belief.city ?? "<city>"}/.sprawling/library/`;
  return (
    <div class="flex flex-col gap-snug text-note text-text-quiet">
      <p>{say("setup_skills")}</p>
      <div class="flex items-center gap-snug">
        <code class="rounded-control bg-g2 px-base py-snug font-mono text-text">{path()}</code>
        <Button
          label={say("setup_copy")}
          tone="quiet"
          onPress={() => {
            void navigator.clipboard.writeText(path());
          }}
        />
      </div>
      <p class="text-text-faint">{say("setup_skills_note")}</p>
    </div>
  );
}

export function Setup() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
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
  const toml = createMemo<string | null>(() => {
    switch (group()) {
      case "accounts":
        return providersToml(answer());
      case "run":
        return modelToml(ui.prefs.effort());
      case "network":
      case "tools":
      case "mcp":
      case "skills":
      case "appearance":
      case "keys":
      case "advanced":
        return null;
    }
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
          <h1 class="text-title font-title">{say(`setup_group_${group()}`)}</h1>
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
              <Match when={group() === "mcp"}>
                <a
                  href={toFragment({ kind: "mcp" })}
                  class="inline-block rounded-control bg-g2 px-base py-snug text-label hover:bg-g3"
                >
                  {say("nav_mcp")}
                </a>
              </Match>
              <Match when={group() === "skills"}>
                <SkillsNote />
              </Match>
              <Match when={group() === "appearance"}>
                <AppearanceSection />
                <div class="flex flex-col gap-tight text-note text-text-quiet">
                  {say("setup_language")}
                  <Segmented
                    label={say("setup_language")}
                    options={LANGS.map((lang) => ({ value: lang, label: endonym(lang) }))}
                    held={ui.prefs.lang()}
                    onPick={(lang) => {
                      ui.prefs.setLang(lang);
                    }}
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

        <Show
          when={toml()}
          fallback={
            <p class="min-w-0 text-note text-text-disabled wide:w-tree wide:shrink-0">
              {say("setup_toml_none")}
            </p>
          }
        >
          {(text) => (
            <div class="min-w-0 wide:w-tree wide:shrink-0">
              <Toml text={text()} />
            </div>
          )}
        </Show>
      </div>
    </div>
  );
}
