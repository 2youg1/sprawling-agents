<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // Which model does the thinking, and how hard by default. `main` is
  // the one a dispatch is refused without; `digest` reads long documents
  // on its behalf and follows `main` unless it is pointed elsewhere;
  // `transcribe` turns a recording into a line of text, and a city with
  // none draws no microphone.
  //
  // One choice per role, each a `parts/combobox.svelte`: an endpoint
  // that answers with two hundred rows is not a list anybody scrolls,
  // and the search box the popup opens with is the control that
  // replaces the arrow (client/Spec.lean §7 names this seat).
  import type { EndpointsAnswer, ModelTag } from "../../wire";

  interface ModelChoiceProps {
    readonly answer: EndpointsAnswer;
    readonly tags?: readonly ModelTag[];
  }
</script>

<script lang="ts">
  import { selectModel } from "../../core/commands";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Combobox from "../parts/combobox.svelte";

  const { answer, tags = ["main", "digest", "transcribe"] }: ModelChoiceProps = $props();
  const { lang, send } = ui();

  // An endpoint is carried by the id a command names it with and read
  // by the name the person gave it. The label is never empty: an
  // endpoint nobody named is labelled with its own id by the city
  // (`wire::EndpointSummary`), so this list needs no fallback.
  const options = $derived(
    answer.endpoints.flatMap((endpoint) =>
      endpoint.models.map((row) => ({
        endpoint: endpoint.name,
        label: endpoint.label,
        model: row.id,
      })),
    ),
  );
  const choices = $derived(
    options.map((option) => ({
      value: `${option.endpoint}\u0000${option.model}`,
      label: option.model,
      note: option.label,
    })),
  );

  function chosen(tag: ModelTag) {
    return answer.chosen.find((each) => each.tag === tag);
  }

  function valueOf(tag: ModelTag): string | null {
    const held = chosen(tag);
    return held === undefined ? null : `${held.endpoint}\u0000${held.model}`;
  }

  function pick(tag: ModelTag, value: string): void {
    const [endpoint, model] = value.split("\u0000");
    if (endpoint === undefined || model === undefined || model === "") return;
    send(selectModel(endpoint, model, tag));
    if (tag === "main" && chosen("digest") === undefined) {
      send(selectModel(endpoint, model, "digest"));
    }
  }
</script>

<div class="flex flex-col gap-base">
  {#if options.length > 0}
    {#each tags as tag (tag)}
      <div class="flex flex-col gap-tight text-note text-text-quiet">
        <span>{say($lang, `setup_${tag}`)}</span>
        <Combobox
          label={say($lang, `setup_${tag}`)}
          placeholder={say($lang, "part_search")}
          empty={say($lang, "part_no_match")}
          {choices}
          value={valueOf(tag)}
          onPick={(value: string) => {
            pick(tag, value);
          }}
        />
      </div>
    {/each}
  {:else}
    <p class="text-note text-text-faint">{say($lang, "setup_no_models")}</p>
  {/if}
</div>
