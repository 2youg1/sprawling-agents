<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Snippet } from "svelte";

  import type { EndpointListLook } from "./endpoint_list";

  export interface EndpointListLookProps extends EndpointListLook {
    // A row's account editor, drawn by the seat under the endpoint's id.
    readonly accounts: Snippet<[string]>;
  }
</script>

<script lang="ts">
  // How the list of attached endpoints is drawn, and nothing else: every
  // word arrives translated (`./endpoint_list`, `EndpointListLook`), and
  // each row's account editor arrives as a snippet, so another look
  // draws the same list by taking the same value.
  import Badge from "../../parts/badge.svelte";
  import EmptyState from "../../parts/empty.svelte";

  const look: EndpointListLookProps = $props();
</script>

{#if look.rows.length > 0}
  <ul class="flex flex-col gap-snug">
    {#each look.rows as row (row.name)}
      <li class="rounded-card bg-chrome px-base py-snug text-note">
        <div class="flex min-w-0 items-center gap-snug">
          <span class="font-label text-text">{row.label}</span>
          {#if row.id !== undefined}
            <span class="font-mono text-text-faint">{row.id}</span>
          {/if}
          {#if row.local !== undefined}
            <Badge text={row.local} />
          {/if}
          <span class="text-text-faint">{row.face}</span>
          <span class="min-w-0 flex-1 truncate font-mono text-text-faint">{row.baseUrl}</span>
          <span class="shrink-0 text-text-faint">{row.keyed}</span>
        </div>
        {#if row.models.length > 0}
          <div class="mt-tight flex flex-wrap gap-tight text-text-quiet">
            {#each row.models as model (model)}
              <span class="rounded-pill bg-raised px-snug font-mono">{model}</span>
            {/each}
          </div>
        {/if}
        <details class="mt-snug">
          <summary class="w-fit cursor-pointer rounded-control px-tight text-text-quiet transition-[background-color] ease-leave hover:bg-raised hover:ease-arrive">{row.accounts}</summary>
          <div class="pt-snug">
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression (a snippet call is the render itself; the typechecker types local snippet calls as returning void) -->
            {@render look.accounts(row.name)}
          </div>
        </details>
      </li>
    {/each}
  </ul>
{:else}
  <EmptyState missing="setup_no_endpoints" seat="region" />
{/if}
