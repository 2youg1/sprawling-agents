<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->
<script lang="ts" module>

  export interface ProviderDoorProps {

    readonly onAttached?: () => void;

    readonly openModels?: boolean;

    readonly stepped?: boolean;
  }
</script>

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Unanswered from "../parts/unanswered.svelte";
  import { AttachForm, EndpointList } from "../setup/providers";

  const { onAttached, openModels = false, stepped = false }: ProviderDoorProps = $props();
  const u = ui();
  const lang = u.lang;

  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const read = $derived(
    readAnswer($endpoints, (held) => ("endpoints" in held ? held.endpoints : undefined)),
  );
</script>

{#snippet listed()}
  {#if read.kind === "held"}
    <EndpointList answer={read.value} {openModels} />
  {:else if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={QUERIES.endpoints} />
  {/if}
{/snippet}

{#if stepped}
  <div class="flex flex-col gap-wide">
    <section class="flex min-w-0 flex-col gap-snug">
      <header class="flex min-w-0 flex-col gap-tight">
        <h3 class="text-label font-label text-text">{say($lang, "setup_step_connected")}</h3>
        <p class="text-note text-text-faint">{say($lang, "setup_step_connected_hint")}</p>
      </header>

      {@render listed()}
    </section>
    <section class="flex min-w-0 max-w-talk flex-col gap-snug border-t border-edge pt-wide">
      <header class="flex min-w-0 flex-col gap-tight">
        <h3 class="text-label font-label text-text">{say($lang, "setup_step_attach")}</h3>
        <p class="text-note text-text-faint">{say($lang, "setup_step_attach_hint")}</p>
      </header>
      <AttachForm {...onAttached === undefined ? {} : { onAttached }} />
    </section>
  </div>
{:else}
  <div class="flex flex-col gap-base">

    {@render listed()}
    <AttachForm {...onAttached === undefined ? {} : { onAttached }} />
  </div>
{/if}
