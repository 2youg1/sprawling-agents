<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // RefRain, the right side's document editor (client/Spec.lean §7N). The
  // three props are the whole door: which building, which path in it,
  // and which version, `null` being the city's current text. A new
  // document or version is a new session, so nothing of one document's
  // draft, receipt or editor survives into the next; the same document
  // drawn again keeps its editor, because the key does not move.
  import { Option, Schema } from "effect";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { Address, type B3Hash } from "../../wire";
  import Document from "./document.svelte";

  interface Props {
    readonly building: Address;
    readonly path: string;
    readonly version: B3Hash | null;
  }

  const { building, path, version }: Props = $props();

  const lang = ui().lang;

  const at = $derived(Option.getOrNull(Schema.decodeOption(Address)(`${building}/${path}`)));
  const key = $derived(`${building}/${path}@${version ?? ""}`);
</script>

{#if at === null}
  <p class="p-pane text-text-faint">{say($lang, "file_missing")}</p>
{:else}
  {#key key}
    <Document {at} {building} {version} />
  {/key}
{/if}
