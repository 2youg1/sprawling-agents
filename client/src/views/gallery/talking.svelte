<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>
  // The states of the Mayor room's box that only show once a person
  // acts: each of the four pills with its menu open, and the card a
  // refused dispatch leaves where the reply would have been.
  //
  // Each menu opens upward from the foot of its case, as it does over
  // the box at the foot of the window, so a case reserves the height
  // of the longest menu above its trigger (`pt-menu` below); a menu
  // drawn outside its case is what `xtask render` refuses.
  import type { AxError } from "../../wire";
  import { Address } from "../../wire";

  const HERE = Address.make("hall/mayor");

  const NO_MODEL: AxError = {
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    action: "choose the main model",
    code: "E_MODEL_UNCHOSEN",
    nearby: [],
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    recovery: "attach a provider on the settings page and pick a model for this tag",
    retry: "no",
    // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
    subject: "no model is chosen for this tag",
  };
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { EFFORTS } from "../../core/commands";
  import { ui } from "../../ui";
  import { pills } from "../talk/composer";
  import PillView from "../talk/pill.svelte";
  import Failed from "../talk/failed.svelte";
  import Case from "./case.svelte";
  import { CHOSEN, MODELS } from "./served";

  const { lang } = ui();

  const ignore = (): void => undefined;
  const specs = $derived(
    pills(
      $lang,
      {
        served: MODELS.map((each) => ({ endpoint: "gallery", model: each.id, label: "gallery" })),
        chosen: { endpoint: "gallery", model: CHOSEN.id },
        session: null,
        rooms: ["hall/mayor", "atlas/api", "atlas/web"],
        here: HERE,
        effort: EFFORTS[2] ?? null,
      },
      { model: ignore, workspace: ignore, effort: ignore },
    ),
  );
  const NAMES = ["workspace"] as const;
</script>

{#each NAMES as name, index (name)}
  {@const spec = specs[index + 1]}
  {#if spec !== undefined}
    <Case label={`composer pill · the ${name} menu open`}>
      <div class="flex min-h-[36rem] items-end">
        <PillView {spec} starts="open" />
      </div>
    </Case>
  {/if}
{/each}

<Case label="conversation · a dispatch refused for want of a model">
  <Failed what={say($lang, "talk_failed_model")} error={NO_MODEL} onRetry={ignore} />
</Case>
