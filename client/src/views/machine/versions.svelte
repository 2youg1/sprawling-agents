<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // One item's three versions on one line: what this machine has, what
  // this repository pins, and the newest upstream, with a mark when the
  // first is behind the last (`views/setup/versions`). A column with
  // nothing to say is left out rather than drawn as a dash.

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { DoctorItem, DoctorNewest } from "../../wire";
  import { versionOf } from "../setup/dependencies";
  import { behind, newestOf } from "../setup/versions";
  import Badge from "../parts/badge.svelte";

  interface Props {
    readonly item: DoctorItem;
    readonly newest: DoctorNewest | undefined;
  }

  const { item, newest }: Props = $props();
  const { lang } = ui();

  const installed = $derived(versionOf(item.state));
  const pinned = $derived(item.pinned ?? null);
  const upstream = $derived(newestOf(newest));
  const stale = $derived("version" in upstream && behind(installed, upstream.version));
  // The newest column's text, and what the network said when the
  // source could not be read.
  const reading = $derived.by((): { readonly version: string | null; readonly phrase: string | null; readonly said: string | null } =>
    "version" in upstream
      ? { version: upstream.version, phrase: null, said: null }
      : { version: null, phrase: say($lang, upstream.key), said: upstream.said },
  );
</script>

<p class="flex min-w-0 flex-wrap items-baseline gap-x-base gap-y-tight text-note">
  {#if installed !== null}
    <!-- A banner with no dotted number in it is shown whole, so it is
    cut at the row's edge rather than pushing its label onto two lines. -->
    <span class="flex min-w-0 max-w-full items-baseline gap-tight">
      <span class="shrink-0 text-text-faint">{say($lang, "machine_version_installed")}</span>
      <span class="min-w-0 truncate font-mono text-text">{installed}</span>
    </span>
  {/if}
  {#if pinned !== null}
    <span class="flex items-baseline gap-tight">
      <span class="text-text-faint">{say($lang, "machine_version_pinned")}</span>
      <span class="font-mono text-text">{pinned}</span>
    </span>
  {/if}
  {#if reading.version !== null}
    <span class="flex items-baseline gap-tight">
      <span class="text-text-faint">{say($lang, "machine_version_newest")}</span>
      <span class="font-mono text-text">{reading.version}</span>
    </span>
  {:else}
    <span class="min-w-0 truncate text-text-faint">{reading.phrase}</span>
  {/if}
  {#if stale}
    <Badge text={say($lang, "machine_version_behind")} weight="live" dot />
  {/if}
  {#if reading.said !== null}
    <span class="min-w-0 basis-full truncate font-mono text-text-faint">{reading.said}</span>
  {/if}
</p>
