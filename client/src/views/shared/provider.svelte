<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The one screen on which a provider is attached: what this city
  // already reaches, and the form a key is entered through.
  //
  // **A key is the only way in.** Subscription quota enters the city
  // through the vendor's own harness, where the person signs in, so
  // this screen offers no login (`crates/gateway/Spec.lean` §8-5).
  //
  // Which endpoints exist is asked here rather than passed in, because
  // `core/asking.ts` matches an answer to a question by content: a
  // page that already asked the same question is answered once and
  // both readers see it.
  //
  // When `stepped`, the list and the form are two numbered stages so a
  // person finishes what is already attached before adding another;
  // the welcome guide keeps the compact stack.

  export interface ProviderDoorProps {
    // A registration went through; the settings page refreshes its
    // lists on it. Absent where there is nothing beside the door to
    // bring up to date.
    readonly onAttached?: () => void;
    // Open required model controls on entry, as the welcome guide does.
    readonly openModels?: boolean;
    // Numbered stages for the settings group's walk-through.
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
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
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
    <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
    {@render listed()}
    <AttachForm {...onAttached === undefined ? {} : { onAttached }} />
  </div>
{/if}
