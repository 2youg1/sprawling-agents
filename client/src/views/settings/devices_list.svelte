<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { DeviceId, DeviceLine } from "../../wire";

  // One row per paired browser: its name, when it paired and was last
  // seen, a mark on the row of the browser drawing the list, and a
  // forget key behind a question, because a forgotten browser cannot
  // come back without the pairing code. A page opened through the
  // remote door says where forgetting is done instead of offering it.
  export interface DevicesLookProps {
    readonly devices: readonly DeviceLine[];
    // This browser's own device id, when it has one.
    readonly here: string | null;
    readonly now: number;
    // Whether this page is on the city's own machine.
    readonly local: boolean;
    readonly onForget: (device: DeviceId) => void;
  }
</script>

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import { ago, clock } from "../../core/time";
  import { ui } from "../../ui";
  import Badge from "../parts/badge.svelte";
  import Button from "../parts/button.svelte";
  import Dialog from "../parts/dialog.svelte";
  import EmptyState from "../parts/empty.svelte";
  import Row, { RowList } from "../parts/row.svelte";

  const { devices, here, now, local, onForget }: DevicesLookProps = $props();
  const { lang } = ui();

  let asking = $state<DeviceLine | null>(null);

  // The name a row is read by. Browsers of one kind pair under one
  // label ("Edge · Windows"), and a compact list draws only the title,
  // so a label two rows share carries when that browser paired: the
  // title and the forget key's name then tell the rows apart.
  const named = $derived.by(() => {
    const shared = new Set(devices.filter((device, at) => devices.findIndex((other) => other.label === device.label) !== at).map((device) => device.label));
    return (device: DeviceLine): string =>
      shared.has(device.label) ? `${device.label} · ${clock($lang, device.paired_at)}` : device.label;
  });

  function seen(device: DeviceLine): string {
    const paired = `${say($lang, "devices_paired_at")} ${ago($lang, device.paired_at, now)}`;
    const last =
      device.last_seen === undefined || device.last_seen === null
        ? say($lang, "devices_never_seen")
        : `${say($lang, "devices_last_seen")} ${ago($lang, device.last_seen, now)}`;
    return `${paired} · ${last}`;
  }
</script>

{#snippet rows()}
  {#each devices as device (device.id)}
    <Row primary={named(device)} secondary={seen(device)}>
      {#snippet status()}
        {#if device.id === here}
          <Badge text={say($lang, "devices_this")} dot />
        {/if}
      {/snippet}
      {#snippet actions()}
        {#if local}
          <Button
            label={fill(say($lang, "devices_forget"), { label: named(device) })}
            tone="quiet"
            onPress={() => {
              asking = device;
            }}
          />
        {/if}
      {/snippet}
    </Row>
  {/each}
{/snippet}

<div class="flex flex-col gap-base">
  {#if !local}
    <p class="text-note text-text-quiet">{say($lang, "devices_local_only")}</p>
  {/if}
  {#if devices.length === 0}
    <EmptyState missing="devices_none" seat="region" />
  {:else}
    <div class="rounded-card border border-edge-panel">
      <!-- eslint-disable-next-line @typescript-eslint/no-unsafe-call (the walk is a snippet exported from a module script; svelte-check types this call, while eslint's own pass cannot see through the component file) -->
      {@render RowList({ label: say($lang, "devices_title"), rows })}
    </div>
  {/if}
</div>

<Dialog
  open={asking !== null}
  title={fill(say($lang, "devices_forget_title"), { label: asking === null ? "" : named(asking) })}
  detail={say($lang, "devices_forget_detail")}
  confirmLabel={fill(say($lang, "devices_forget"), { label: asking === null ? "" : named(asking) })}
  cancelLabel={say($lang, "quit_cancel")}
  destructive
  onConfirm={() => {
    if (asking !== null) onForget(asking.id);
    asking = null;
  }}
  onCancel={() => {
    asking = null;
  }}
/>
