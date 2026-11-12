<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One doctor item's name, linked to its release or installation page
  // when the table names one. The link opens in a new browsing context
  // so the doctor page stays where it is.
  import type { DoctorItem } from "../../wire";
  import Tip from "../parts/tip.svelte";

  import { lookOf } from "./doctor_link";

  interface Props {
    readonly item: DoctorItem;
  }

  const { item }: Props = $props();
  const look = $derived(lookOf(item));
</script>

{#if look.homepage === null}
  <span class="shrink-0 whitespace-nowrap font-mono text-label text-text">{look.name}</span>
{:else}
  <span class="shrink-0 whitespace-nowrap"
    ><Tip text={look.homepage}>
      {#snippet children(hint)}
        <a
          class="font-mono text-label text-text underline decoration-edge underline-offset-2 transition-colors ease-leave hover:decoration-accent hover:ease-arrive"
          href={look.homepage}
          target={look.target}
          rel="noreferrer"
          aria-describedby={hint}
        >
          {look.name}
        </a>
      {/snippet}
    </Tip></span
  >
{/if}
