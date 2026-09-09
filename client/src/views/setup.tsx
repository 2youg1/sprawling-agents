// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Settings: what is attached, which model thinks, how hard by default,
// who answers approvals, which language, and the way back into the
// welcome. One column, one section per question.

import { For, Show, createMemo, createSignal } from "solid-js";
import type { JSX } from "solid-js";

import { configureMcp, setAutonomy } from "../core/commands";
import { LANGS, endonym } from "../core/lang";
import { toFragment } from "../core/route";
import type { Autonomy, McpServer } from "../wire";
import { Address as AddressSchema, ResidentId, ServerLabel } from "../wire";
import { useCommand, useSay, useUi } from "../ui";
import { EffortChoice, ModelChoice } from "./setup/models";
import { AttachForm, EndpointList, LoginForm } from "./setup/providers";

function Section(props: { readonly title: string; readonly children: JSX.Element }) {
  return (
    <section class="border-t border-g1 py-wide">
      <h2 class="mb-base text-heading font-heading">{props.title}</h2>
      {props.children}
    </section>
  );
}

const HALL = AddressSchema.make("hall");

export function McpForm() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const [label, setLabel] = createSignal("");
  const [command_, setCommandLine] = createSignal("");
  const [url, setUrl] = createSignal("");
  const hall = ui.conn.asking.ask({ building_view: { addr: HALL } });
  const servers = createMemo<readonly McpServer[]>(() => {
    const answer = hall();
    return answer !== undefined && "building" in answer ? answer.building.mcp : [];
  });
  const add = () => {
    const name = label().trim();
    if (name === "" || !/^[a-z][a-z0-9-]*$/.test(name)) return;
    const transport: McpServer["transport"] =
      url().trim() !== ""
        ? { http: { url: url().trim(), header: null } }
        : (() => {
            const [program, ...args] = command_().trim().split(/\s+/);
            return { stdio: { command: program ?? "", args } };
          })();
    if ("stdio" in transport && transport.stdio.command === "") return;
    if (command(configureMcp(HALL, [...servers(), { label: ServerLabel.make(name), transport }]))) {
      setLabel("");
      setCommandLine("");
      setUrl("");
    }
  };
  return (
    <div class="flex flex-col gap-base">
      <Show when={servers().length > 0}>
        <ul class="flex flex-col gap-tight text-note">
          <For each={servers()}>
            {(server) => (
              <li class="flex items-center gap-base rounded-card bg-g1 px-base py-snug">
                <span class="font-label text-text">{server.label}</span>
                <span class="flex-1 truncate font-mono text-text-faint">
                  {"stdio" in server.transport
                    ? [server.transport.stdio.command, ...server.transport.stdio.args].join(" ")
                    : server.transport.http.url}
                </span>
                <button
                  type="button"
                  class="rounded-control px-snug py-tight text-text-faint hover:bg-g2 hover:text-alert"
                  onClick={() => command(configureMcp(HALL, servers().filter((each) => each.label !== server.label)))}
                >
                  {say("setup_remove")}
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>
      <div class="grid gap-snug md:grid-cols-3">
        <input
          class="rounded-control bg-g2 px-base py-snug text-body outline-none placeholder:text-text-disabled"
          placeholder={say("setup_mcp_label")}
          value={label()}
          onInput={(event) => setLabel(event.currentTarget.value)}
        />
        <input
          class="rounded-control bg-g2 px-base py-snug font-mono text-body outline-none placeholder:text-text-disabled"
          placeholder={say("setup_mcp_command")}
          value={command_()}
          onInput={(event) => setCommandLine(event.currentTarget.value)}
        />
        <input
          class="rounded-control bg-g2 px-base py-snug font-mono text-body outline-none placeholder:text-text-disabled"
          placeholder={say("setup_mcp_url")}
          value={url()}
          onInput={(event) => setUrl(event.currentTarget.value)}
        />
      </div>
      <div>
        <button type="button" class="rounded-control bg-g2 px-base py-snug text-label hover:bg-g3" onClick={add}>
          {say("setup_mcp_add")}
        </button>
      </div>
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
        <button
          type="button"
          class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g2"
          onClick={() => {
            void navigator.clipboard.writeText(path());
          }}
        >
          {say("setup_copy")}
        </button>
      </div>
      <p class="text-text-faint">{say("setup_skills_note")}</p>
    </div>
  );
}

export function Setup() {
  const ui = useUi();
  const say = useSay();
  const command = useCommand();
  const endpoints = ui.conn.asking.ask("endpoint_view");
  const answer = createMemo(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });
  const governance = ui.conn.asking.ask("governance");
  const autonomy = createMemo<Autonomy | undefined>(() => {
    const held = governance();
    return held !== undefined && "governance" in held ? held.governance.autonomy : undefined;
  });
  const autonomyKey = (a: Autonomy | undefined) => (a === undefined ? "" : typeof a === "string" ? a : "delegate");
  const [door, setDoor] = createSignal<"key" | "login">("key");

  return (
    <div class="mx-auto w-full max-w-measure px-pane py-wide">
      <h1 class="mb-wide text-title font-title">{say("nav_settings")}</h1>

      <Section title={say("setup_providers")}>
        <Show when={answer()}>{(held) => <EndpointList answer={held()} />}</Show>
        <div class="mt-base flex gap-snug text-label">
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
        <div class="mt-base">
          <Show when={door() === "key"} fallback={<LoginForm />}>
            <AttachForm />
          </Show>
        </div>
      </Section>

      <Section title={say("setup_models")}>
        <Show when={answer()}>{(held) => <ModelChoice answer={held()} />}</Show>
        <div class="mt-base">
          <EffortChoice />
        </div>
      </Section>

      <Section title={say("setup_autonomy")}>
        <div class="flex flex-wrap gap-tight">
          <For
            each={[
              ["owner", say("autonomy_owner")],
              ["delegate", say("autonomy_clerk")],
              ["deferred", say("autonomy_deferred")],
            ] satisfies [string, string][]}
          >
            {([key, label]) => (
              <button
                type="button"
                class={`rounded-pill px-base py-tight text-label ${autonomyKey(autonomy()) === key ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"}`}
                onClick={() =>
                  command(
                    setAutonomy(
                      "city",
                      key === "owner" ? "owner" : key === "deferred" ? "deferred" : { delegate: ResidentId.make("hall/clerk") },
                    ),
                  )
                }
              >
                {label}
              </button>
            )}
          </For>
        </div>
      </Section>

      <Section title={say("setup_mcp")}>
        <McpForm />
      </Section>

      <Section title={say("setup_skills_title")}>
        <SkillsNote />
      </Section>

      <Section title={say("setup_language")}>
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
      </Section>

      <Section title={say("setup_again")}>
        <a href={toFragment({ kind: "welcome" })} class="rounded-control bg-g2 px-base py-snug text-label hover:bg-g3">
          {say("setup_rerun")}
        </a>
      </Section>
    </div>
  );
}
