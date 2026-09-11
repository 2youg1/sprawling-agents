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
// A group whose section another view owns mounts that view; a group
// whose section does not exist yet says so by name rather than drawing
// an empty screen. Today that is `network`: the proxy the city would
// use is neither on the wire nor in a view, so the screen names what is
// missing instead of drawing an empty form.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";
import type { JSX } from "solid-js";

import { setAutonomy } from "../core/commands";
import { LANGS, endonym } from "../core/lang";
import { toFragment } from "../core/route";
import type { Autonomy, EndpointsAnswer } from "../wire";
import { ResidentId } from "../wire";
import { useCommand, useSay, useUi } from "../ui";
import { Machine } from "./machine";
import { Button } from "./parts/button";
import { KeysSection } from "./setup/keys";
import { AppearanceSection } from "./setup/appearance";
import { EffortChoice, ModelChoice } from "./setup/models";
import { AttachForm, EndpointList, LoginForm } from "./setup/providers";

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

// How hard the city thinks by default, and who answers an approval.
function runToml(effort: string, autonomy: Autonomy | undefined): string | null {
  if (autonomy === undefined) {
    return null;
  }
  const approvals = typeof autonomy === "string" ? autonomy : `delegate:${autonomy.delegate}`;
  return ["[run]", `effort = "${effort}"`, `approvals = "${approvals}"`].join("\n");
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
        <code class="whitespace-pre-wrap break-all font-mono text-note text-text-quiet">{props.text}</code>
      </pre>
    </aside>
  );
}

// A screen somebody else is building, named so a person knows what is
// missing rather than meeting a blank column.
function Slot(props: { readonly what: string }) {
  const say = useSay();
  return <p class="text-note text-text-disabled">{say("setup_slot_pending", { what: props.what })}</p>;
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

function Providers(props: { readonly answer: EndpointsAnswer | undefined }) {
  const say = useSay();
  const [door, setDoor] = createSignal<"key" | "login">("key");
  return (
    <div class="flex flex-col gap-base">
      <Show when={props.answer}>{(held) => <EndpointList answer={held()} />}</Show>
      <div class="flex gap-snug text-label">
        <button
          type="button"
          class={`rounded-pill px-base py-tight ${door() === "key" ? "bg-g3 text-text" : "text-text-faint hover:text-text-quiet"}`}
          onClick={() => setDoor("key")}
        >
          {say("setup_attach")}
        </button>
        <button
          type="button"
          class={`rounded-pill px-base py-tight ${door() === "login" ? "bg-g3 text-text" : "text-text-faint hover:text-text-quiet"}`}
          onClick={() => setDoor("login")}
        >
          {say("setup_login")}
        </button>
      </div>
      <Show when={door() === "key"} fallback={<LoginForm />}>
        <AttachForm />
      </Show>
      <h2 class="text-label font-label text-text-quiet">{say("setup_models")}</h2>
      <Show when={props.answer}>{(held) => <ModelChoice answer={held()} />}</Show>
    </div>
  );
}

export function Setup() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [group, setGroup] = createSignal<Group>("accounts");
  const endpoints = ui.conn.asking.ask("endpoint_view");
  const answer = createMemo<EndpointsAnswer | undefined>(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });
  const governance = ui.conn.asking.ask("governance");
  const autonomy = createMemo<Autonomy | undefined>(() => {
    const held = governance();
    return held !== undefined && "governance" in held ? held.governance.autonomy : undefined;
  });
  const autonomyKey = (a: Autonomy | undefined) =>
    a === undefined ? "" : typeof a === "string" ? a : "delegate";
  const toml = createMemo<string | null>(() => {
    switch (group()) {
      case "accounts":
        return providersToml(answer());
      case "run":
        return runToml(ui.prefs.effort(), autonomy());
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
    <div class="flex min-h-0 w-full flex-1 flex-col-reverse gap-wide px-pane py-wide lg:flex-row">
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
          <div class={group() === "tools" ? "min-w-0" : "min-w-0 max-w-measure"}>
            <Switch>
              <Match when={group() === "accounts"}>
                <Providers answer={answer()} />
              </Match>
              <Match when={group() === "run"}>
                <div class="flex flex-col gap-wide">
                  <EffortChoice />
                  <div class="flex flex-col gap-tight text-note text-text-quiet">
                    {say("setup_autonomy")}
                    <div class="flex flex-wrap gap-tight">
                      <For
                        each={
                          [
                            ["owner", say("autonomy_owner")],
                            ["delegate", say("autonomy_clerk")],
                            ["deferred", say("autonomy_deferred")],
                          ] satisfies [string, string][]
                        }
                      >
                        {([key, label]) => (
                          <button
                            type="button"
                            class={`rounded-pill px-base py-tight text-label ${autonomyKey(autonomy()) === key ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"}`}
                            onClick={() =>
                              command(
                                setAutonomy(
                                  "city",
                                  key === "owner"
                                    ? "owner"
                                    : key === "deferred"
                                      ? "deferred"
                                      : { delegate: ResidentId.make("hall/clerk") },
                                ),
                              )
                            }
                          >
                            {label}
                          </button>
                        )}
                      </For>
                    </div>
                  </div>
                </div>
              </Match>
              <Match when={group() === "network"}>
                <Slot what={say("setup_group_network")} />
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
                  <div class="flex gap-tight">
                    <For each={LANGS}>
                      {(lang) => (
                        <button
                          type="button"
                          class={`rounded-pill px-base py-tight text-label ${ui.prefs.lang() === lang ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"}`}
                          onClick={() => {
                            ui.prefs.setLang(lang);
                          }}
                        >
                          {endonym(lang)}
                        </button>
                      )}
                    </For>
                  </div>
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
