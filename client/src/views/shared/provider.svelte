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
  // this screen offers no login (gateway-SPEC 8-5).
  //
  // Which endpoints exist is asked here rather than passed in, because
  // `core/asking.ts` matches an answer to a question by content: a
  // page that already asked the same question is answered once and
  // both readers see it.

  export interface ProviderDoorProps {
    // A registration went through; the settings page refreshes its
    // lists on it. Absent where there is nothing beside the door to
    // bring up to date.
    readonly onAttached?: () => void;
  }
</script>

<script lang="ts">
  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { ui } from "../../ui";
  import Unanswered from "../parts/unanswered.svelte";
  import { AttachForm, EndpointList } from "../setup/providers";

  const { onAttached }: ProviderDoorProps = $props();
  const u = ui();

  const endpoints = u.conn.asking.ask(QUERIES.endpoints);
  const read = $derived(readAnswer($endpoints, (held) => ("endpoints" in held ? held.endpoints : undefined)));
</script>

<div class="flex flex-col gap-base">
  {#if read.kind === "held"}
    <EndpointList answer={read.value} />
  {:else if read.kind === "unavailable"}
    <Unanswered query={read.query} asked={QUERIES.endpoints} />
  {/if}
  <AttachForm {...(onAttached === undefined ? {} : { onAttached })} />
</div>
