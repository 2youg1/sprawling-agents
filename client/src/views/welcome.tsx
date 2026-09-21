// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The first half hour: four steps, one decision each, in the order a
// city needs them - this machine, a provider, what is optional, then
// the model and how hard it thinks. The city decides when setup is
// needed (no `main` model); this only walks it.
//
// There used to be a fifth step. It held one conditional warning and
// the button into the Mayor's office, so a person who had just chosen
// a model was made to press `next` to reach a page that told them
// nothing. The button now finishes the fourth step, and the three
// sentences about how the city is used open the Mayor's office itself,
// where they are read while the thing they describe is on screen.
//
// The provider screen and the thinking-effort screen are the same two
// components the settings page mounts (`views/shared/`), so the two
// pages cannot drift apart again.

import { For, Match, Show, Switch, createMemo, createSignal } from "solid-js";

import { QUERIES } from "../core/asking";
import { MAYOR, buildingOf, toFragment } from "../core/route";
import { useGo, useSay, useUi } from "../ui";
import { Machine } from "./machine";
import { McpForm } from "./mcp";
import { Shelves } from "./setup/skills";
import { ModelChoice } from "./setup/models";
import { EffortSection } from "./shared/effort";
import { ProviderDoor } from "./shared/provider";

const STEPS = ["machine", "provider", "optional", "model"] as const;
type Step = (typeof STEPS)[number];

// The last step, named once: it carries the button that ends the walk
// rather than the button that advances it.
const LAST: Step = "model";

export function Welcome() {
  const ui = useUi();
  const say = useSay();
  const go = useGo();
  const [step, setStep] = createSignal<Step>("machine");
  const endpoints = ui.conn.asking.ask(QUERIES.endpoints);
  const answer = createMemo(() => {
    const held = endpoints();
    return held !== undefined && "endpoints" in held ? held.endpoints : undefined;
  });
  const attached = () => (answer()?.endpoints.length ?? 0) > 0;
  const at = () => STEPS.indexOf(step());
  const next = () => setStep(STEPS[Math.min(at() + 1, STEPS.length - 1)] ?? LAST);
  const back = () => setStep(STEPS[Math.max(at() - 1, 0)] ?? "machine");
  const finish = () => {
    ui.prefs.setWelcomed(true);
    go({ kind: "talk", address: MAYOR });
  };

  return (
    <div class="mx-auto flex w-full max-w-page flex-1 gap-section px-pane py-section wide:max-w-none">
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
      <div class={`flex min-w-0 flex-1 flex-col ${step() === "machine" ? "" : "max-w-measure"}`}>
        <p class="text-note text-text-faint">{say("welcome_title")}</p>
        <h1 class="mb-wide mt-tight text-title font-title">{say(`welcome_step_${step()}`)}</h1>
        <div>
          <Switch>
            <Match when={step() === "machine"}>
              <Machine />
            </Match>
            <Match when={step() === "provider"}>
              <ProviderDoor />
            </Match>
            <Match when={step() === "optional"}>
              <h2 class="mb-base text-label font-label text-text-quiet">{say("setup_mcp")}</h2>
              <McpForm addr={buildingOf(MAYOR)} />
              <h2 class="mt-wide mb-base text-label font-label text-text-quiet">{say("setup_skills_title")}</h2>
              <Shelves />
            </Match>
            <Match when={step() === LAST}>
              <Show when={answer()}>{(held) => <ModelChoice answer={held()} />}</Show>
              <div class="mt-wide">
                <EffortSection />
              </div>
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
            when={step() !== LAST}
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
