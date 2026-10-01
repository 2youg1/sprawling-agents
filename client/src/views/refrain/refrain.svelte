<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // RefRain, the right side's document editor (client-SPEC 7F). Until the
  // editor lands it shows the document through the building's file view,
  // which reads the worktree's current text; `version` is part of the
  // door already, so every opener names the version it means.
  import { Option, Schema } from "effect";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { Address, type B3Hash } from "../../wire";
  import FileView from "../building/file.svelte";

  interface Props {
    readonly building: Address;
    readonly path: string;
    readonly version: B3Hash | null;
  }

  // eslint-disable-next-line svelte/no-unused-props -- the stub reads the worktree's current text; the editor reads `version`
  const { building, path }: Props = $props();

  const lang = ui().lang;

  const at = $derived(Option.getOrNull(Schema.decodeOption(Address)(`${building}/${path}`)));
</script>

{#if at === null}
  <p class="p-pane text-text-faint">{say($lang, "file_missing")}</p>
{:else}
  <FileView {at} root={building} />
{/if}
