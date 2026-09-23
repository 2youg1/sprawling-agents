<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // What is attached, one row each - the list that stands over both
  // doors a provider is reached through. The subscription door and the
  // door picker are `shared/provider.svelte`'s own now, and the key
  // door is `providers/form.svelte`; this file is what came in
  // through them, and `providers.ts` is the one specifier a caller
  // reaches the pair through.
  //
  // **The row is headed by the name the person gave it.** `label` is
  // the display name, and the city answers with the id when nobody
  // stated one, so this row never has to decide what to show. The id
  // appears beside it only when the two differ, because that is when a
  // person needs both: the label to recognise the provider, and the id
  // to find its table in a `config.toml` and its key in the vault.
  //
  // **The badge says the call never leaves this machine.** The city
  // decides that from the base URL and states it as `local`, and a
  // confidential building is refused every endpoint without it
  // (`gateway::router::book::select`), so it is the one property of a
  // row that changes what a person may do with it.

  import type { EndpointsAnswer } from "../../wire";

  export interface EndpointListProps {
    readonly answer: EndpointsAnswer;
  }
</script>

<script lang="ts">
  import { wireApiOf } from "../../core/commands";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import EmptyState from "../parts/empty.svelte";

  const { answer }: EndpointListProps = $props();
  const { lang } = ui();
</script>

{#if answer.endpoints.length > 0}
  <ul class="flex flex-col gap-snug">
    {#each answer.endpoints as endpoint (endpoint.name)}
      <li class="rounded-card bg-chrome px-base py-snug text-note">
        <div class="flex items-center gap-snug">
          <span class="font-label text-text">{endpoint.label}</span>
          {#if endpoint.label !== endpoint.name}
            <span class="font-mono text-text-faint">{endpoint.name}</span>
          {/if}
          {#if endpoint.local}
            <Badge text={say($lang, "setup_local")} />
          {/if}
          <!-- wording-ok: the wire value `chat` / `responses` / `messages`, spelled the same in both languages -->
          <span class="text-text-faint">{wireApiOf(endpoint.dialect)}</span>
          <span class="flex-1 truncate font-mono text-text-disabled">{endpoint.base_url}</span>
          <span class="text-text-faint">
            {endpoint.has_credential ? say($lang, "setup_keyed") : say($lang, "setup_unkeyed")}
          </span>
        </div>
        <div class="mt-tight flex flex-wrap gap-tight text-text-quiet">
          {#each endpoint.models as model (model.id)}
            <span class="rounded-pill bg-raised px-snug">{model.id}</span>
          {/each}
        </div>
      </li>
    {/each}
  </ul>
{:else}
  <EmptyState missing="setup_no_endpoints" />
{/if}
