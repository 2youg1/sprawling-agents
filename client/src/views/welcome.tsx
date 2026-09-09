// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The first half hour: five steps, one decision each, in the order a
// city needs them - this machine, a provider, what is optional, the
// model and how hard it thinks, then the Mayor. The city decides when
// setup is needed (no `main` model); this only walks it.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import { MAYOR, buildingOf, toFragment } from "../core/route";
import { useGo, useSay, useUi } from "../ui";
import { McpForm } from "./mcp";
import { SkillsNote } from "./setup";
import { EffortChoice, ModelChoice } from "./setup/models";
import { AttachForm, EndpointList, LoginForm } from "./setup/providers";

const STEPS = ["machine", "provider", "optional", "model", "go"] as const;
type Step = (typeof STEPS)[number];

const DOCTOR = "sprawling doctor --install";

export function Welcome() {
  const ui = useUi();
  const say = useSay();
  const go = useGo();
  const [step, setStep] = createSignal<Step>("machine");
  const [door, setDoor] = createSignal<"key" | "login">("key");
  const endpoints = ui.conn.asking.ask("endpoint_view");
  const answer = createMemo(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });
  const attached = () => (answer()?.endpoints.length ?? 0) > 0;
  const chosen = () => answer()?.chosen.some((each) => each.tag === "main") ?? false;
  const at = () => STEPS.indexOf(step());
  const next = () => setStep(STEPS[Math.min(at() + 1, STEPS.length - 1)] ?? "go");
  const back = () => setStep(STEPS[Math.max(at() - 1, 0)] ?? "machine");
  const finish = () => {
    ui.prefs.setWelcomed(true);
    go({ kind: "talk", address: MAYOR });
  };

  return (
    <div class="mx-auto flex w-full max-w-page flex-1 gap-section px-pane py-section">
      <ol class="hidden w-tree shrink-0 flex-col gap-base pt-step text-label md:flex" aria-label={say("welcome_steps")}>
        <For each={STEPS}>
          {(each, index) => (
            <li class={`flex items-center gap-base ${step() === each ? "text-text" : index() < at() ? "text-text-faint" : "text-text-disabled"}`}>
              <span class={`flex size-step items-center justify-center rounded-pill text-note ${step() === each ? "bg-accent text-g0" : "bg-g1"}`}>
                {index() + 1}
              </span>
              {say(`welcome_step_${each}`)}
            </li>
          )}
        </For>
      </ol>
      <div class="flex min-w-0 max-w-measure flex-1 flex-col">
        <p class="text-note text-text-faint">{say("welcome_title")}</p>
        <h1 class="mb-wide mt-tight text-title font-title">{say(`welcome_step_${step()}`)}</h1>
        <div>
          <Switch>
            <Match when={step() === "machine"}>
              <p class="mb-base text-note text-text-faint">{say("welcome_machine_body")}</p>
              <div class="flex items-center gap-snug">
                <code class="rounded-control bg-g1 px-base py-snug font-mono text-text">{DOCTOR}</code>
                <button
                  type="button"
                  class="rounded-control px-snug py-tight text-label text-text-quiet hover:bg-g1"
                  onClick={() => {
                    void navigator.clipboard.writeText(DOCTOR);
                  }}
                >
                  {say("setup_copy")}
                </button>
              </div>
            </Match>
            <Match when={step() === "provider"}>
              <Show when={answer()}>{(held) => <div class="mb-base"><EndpointList answer={held()} /></div>}</Show>
              <div class="mb-base flex gap-snug text-label">
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
            </Match>
            <Match when={step() === "optional"}>
              <h2 class="mb-base text-label font-label text-text-quiet">{say("setup_mcp")}</h2>
              <McpForm addr={buildingOf(MAYOR)} />
              <h2 class="mt-wide mb-base text-label font-label text-text-quiet">{say("setup_skills_title")}</h2>
              <SkillsNote />
            </Match>
            <Match when={step() === "model"}>
              <Show when={answer()}>{(held) => <ModelChoice answer={held()} />}</Show>
              <div class="mt-wide">
                <EffortChoice />
              </div>
            </Match>
            <Match when={step() === "go"}>
              <Show when={!chosen()}>
                <p class="mb-base text-note text-alert">{say("welcome_no_main")}</p>
              </Show>
            </Match>
          </Switch>
        </div>
        <div class="mt-wide flex items-center gap-snug text-label">
          <Show when={at() > 0}>
            <button type="button" class="rounded-control px-base py-snug text-text-quiet hover:bg-g1" onClick={back}>
              {say("welcome_back")}
            </button>
          </Show>
          <span class="flex-1" />
          <Show when={step() === "optional"}>
            <button type="button" class="rounded-control px-base py-snug text-text-faint hover:bg-g1" onClick={next}>
              {say("welcome_skip")}
            </button>
          </Show>
          <Show
            when={step() !== "go"}
            fallback={
              <button type="button" class="rounded-control bg-accent px-wide py-snug text-g0 hover:bg-accent-hover" onClick={finish}>
                {say("welcome_done")}
              </button>
            }
          >
            <button
              type="button"
              class="rounded-control bg-accent px-wide py-snug text-g0 hover:bg-accent-hover disabled:bg-g3 disabled:text-text-disabled"
              disabled={step() === "provider" && !attached()}
              onClick={next}
            >
              {say("welcome_next")}
            </button>
          </Show>
        </div>
        <a href={toFragment({ kind: "talk", address: MAYOR })} class="mt-base text-note text-text-disabled" onClick={() => {
            ui.prefs.setWelcomed(true);
          }}
        >
          {say("welcome_later")}
        </a>
      </div>
    </div>
  );
}

