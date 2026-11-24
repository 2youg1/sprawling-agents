// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, For, Show } from "solid-js";

import { selectModel } from "../../core/commands";
import { say, type Key } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import type { EndpointsAnswer, ModelTag } from "../../wire";
import Combobox from "../parts/combobox_view";

interface Props {
  readonly answer: EndpointsAnswer;
  readonly tags?: readonly ModelTag[];
}

const TAGS: readonly ModelTag[] = ["main", "digest", "transcribe"];
const KEY_SEPARATOR = "\u0000";

export default function ModelChoice(props: Props) {
  const { lang, send } = ui();
  const language = storeValue(lang);
  const words = (key: Key) => say(language(), key);
  const choices = createMemo(() =>
    props.answer.endpoints.flatMap((endpoint) =>
      endpoint.models.map((model) => ({
        value: `${endpoint.name}${KEY_SEPARATOR}${model.id}`,
        label: model.id,
        note: endpoint.label,
      })),
    ),
  );

  function chosen(tag: ModelTag) {
    return props.answer.chosen.find((model) => model.tag === tag);
  }

  function valueOf(tag: ModelTag): string | null {
    const held = chosen(tag);
    return held === undefined ? null : `${held.endpoint}${KEY_SEPARATOR}${held.model}`;
  }

  function pick(tag: ModelTag, value: string): void {
    const [endpoint, model] = value.split(KEY_SEPARATOR);
    if (endpoint === undefined || model === undefined || model === "") return;
    send(selectModel(endpoint, model, tag));
    if (tag === "main" && chosen("digest") === undefined) {
      send(selectModel(endpoint, model, "digest"));
    }
  }

  return (
    <div class="flex flex-col gap-base">
      <Show
        when={choices().length > 0}
        fallback={<p class="text-note text-text-faint">{words("setup_no_models")}</p>}
      >
        <For each={props.tags ?? TAGS}>
          {(tag) => (
            <div class="flex flex-col gap-tight text-note text-text-quiet">
              <span>{words(`setup_${tag}`)}</span>
              <Combobox
                label={words(`setup_${tag}`)}
                placeholder={words("part_search")}
                empty={words("part_no_match")}
                choices={choices()}
                value={valueOf(tag)}
                onPick={(value) => pick(tag, value)}
              />
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
