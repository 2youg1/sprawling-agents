<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address, B3Hash } from "../../wire";
  import Document from "./document.svelte";
  import { documentAt } from "./session.svelte";

  interface Props {
    readonly building: Address;
    readonly path: string;
    readonly version: B3Hash | null;
  }

  const { building, path, version }: Props = $props();

  const lang = ui().lang;

  const at = $derived(documentAt(building, path));
  const key = $derived(`${building}/${path}@${version ?? ""}`);
</script>

{#if at === null}
  <p class="p-pane text-text-faint">{say($lang, "file_missing")}</p>
{:else}
  {#key key}
    <Document {at} {building} {version} />
  {/key}
{/if}
