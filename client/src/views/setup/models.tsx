// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which model does the thinking, and how hard by default. `main` is the
// one a dispatch is refused without; `digest` reads long documents on
// its behalf and follows `main` unless it is pointed elsewhere;
// `transcribe` turns a recording into a line of text, and a city with
// none draws no microphone.

import { For, Show, createMemo } from "solid-js";

import { selectModel } from "../../core/commands";
import { EFFORTS } from "../../core/prefs";
import type { EndpointsAnswer, ModelTag } from "../../wire";
import { useCommand, useSay, useUi } from "../../ui";

export function ModelChoice(props: { readonly answer: EndpointsAnswer; readonly tags?: readonly ModelTag[] }) {
  const say = useSay();
  const command = useCommand();
  const tags = () => props.tags ?? (["main", "digest", "transcribe"] as const);
  const options = createMemo(() =>
    props.answer.endpoints.flatMap((endpoint) => endpoint.models.map((model) => ({ endpoint: endpoint.name, model }))),
  );
  const chosen = (tag: ModelTag) => props.answer.chosen.find((each) => each.tag === tag);
  const value = (tag: ModelTag) => {
    const held = chosen(tag);
    return held === undefined ? "" : `${held.endpoint}\u0000${held.model}`;
  };
  return (
    <div class="flex flex-col gap-base">
      <Show when={options().length > 0} fallback={<p class="text-note text-text-faint">{say("setup_no_models")}</p>}>
        <For each={tags()}>
          {(tag) => (
            <label class="flex flex-col gap-tight text-note text-text-quiet">
              {say(`setup_${tag}`)}
              <select
                class="rounded-control bg-g2 px-base py-snug text-body text-text outline-none"
                value={value(tag)}
                onChange={(event) => {
                  const [endpoint, model] = event.currentTarget.value.split("\u0000");
                  if (endpoint !== undefined && model !== undefined && model !== "") {
                    command(selectModel(endpoint, model, tag));
                    if (tag === "main" && chosen("digest") === undefined) {
                      command(selectModel(endpoint, model, "digest"));
                    }
                  }
                }}
              >
                <option value="">—</option>
                <For each={options()}>
                  {(option) => (
                    <option value={`${option.endpoint}\u0000${option.model}`}>
                      {option.model} · {option.endpoint}
                    </option>
                  )}
                </For>
              </select>
            </label>
          )}
        </For>
      </Show>
    </div>
  );
}

export function EffortChoice() {
  const ui = useUi();
  const say = useSay();
  return (
    <div class="flex flex-col gap-tight text-note text-text-quiet">
      {say("setup_effort")}
      <div class="flex flex-wrap gap-tight">
        <For each={EFFORTS}>
          {(effort) => (
            <button
              type="button"
              class={`rounded-pill px-base py-tight text-label ${ui.prefs.effort() === effort ? "bg-accent text-g0" : "bg-g2 text-text-quiet hover:bg-g3"}`}
              onClick={() => {
                ui.prefs.setEffort(effort);
              }}
            >
              {say(`effort_${effort}`)}
            </button>
          )}
        </For>
      </div>
    </div>
  );
}
