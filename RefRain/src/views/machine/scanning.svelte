<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The scan in front of the city's directory, one line per fact the
  // city read (`scanning.ts`): on Windows the directory, the Dev Drive
  // and Defender's exclusion; on macOS and Linux the reason there is
  // nothing to read.

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { DoctorScanning } from "../../wire";
  import { scanningOf } from "./scanning";

  interface Props {
    readonly scanning: DoctorScanning;
  }

  const { scanning }: Props = $props();
  const { lang } = ui();
</script>

<section class="flex min-w-0 flex-col gap-tight" aria-label={say($lang, "machine_scan_title")}>
  <h2 class="text-label font-label text-text">{say($lang, "machine_scan_title")}</h2>
  <dl class="grid grid-cols-[minmax(0,10rem)_minmax(0,1fr)] gap-x-base gap-y-tight text-note">
    {#each scanningOf(scanning) as line (line.subject)}
      <dt class="text-text-faint">{say($lang, line.subject)}</dt>
      <dd class="flex min-w-0 flex-wrap items-baseline gap-tight">
        <span class="text-text">{say($lang, line.found.key)}</span>
        {#if line.found.said !== null}
          <span class="min-w-0 break-all font-mono text-text-faint">{line.found.said}</span>
        {/if}
      </dd>
    {/each}
  </dl>
</section>
