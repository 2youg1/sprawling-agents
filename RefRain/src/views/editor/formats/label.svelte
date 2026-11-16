<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { fill, say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import type { B3Hash } from "../../../wire";
  import { short } from "../reading";
  import { NAME } from "./format";
  import type { Drawn } from "./format";

  interface Props {
    readonly format: Drawn;
    readonly tool: string;
    readonly settings: readonly string[];

    readonly version: B3Hash | null;
    readonly notes?: readonly string[];
  }

  const { format, tool, settings, version, notes = [] }: Props = $props();

  const lang = ui().lang;

  const line = $derived(
    [
      tool,
      ...settings,
      ...(version === null
        ? []
        : [fill(say($lang, "format_version"), { version: short(version) })]),
    ].join(" · "),
  );
</script>

<div class="border-b border-edge px-wide py-tight text-note text-text-faint">
  <p><span class="text-text-quiet">{NAME[format]}</span> · {line}</p>
  {#each notes as note (note)}
    <p>{note}</p>
  {/each}
</div>
