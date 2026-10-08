<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The paired browsers group (client/Spec.lean §4-57b): asks the city
  // for every browser that paired at this machine's door and hands the
  // answer to the list (`devices_list.svelte`), which the gallery draws
  // from a fixture. Forgetting one is `ForgetDevice`, a verb only this
  // machine's own browser may send.
  import { onMount } from "svelte";

  import { QUERIES } from "../../core/asking";
  import { readAnswer } from "../../core/answered";
  import { forgetDevice } from "../../core/commands";
  import { kept } from "../../core/local/device";
  import { ui } from "../../ui";
  import type { DeviceId } from "../../wire";
  import Unanswered from "../parts/unanswered.svelte";
  import Look from "./devices_list.svelte";

  const u = ui();
  const asked = u.conn.asking.ask(QUERIES.devices);
  const read = $derived(readAnswer($asked, (held) => ("devices" in held ? held.devices.devices : undefined)));
  // This browser's own id, so its row can say so.
  let here = $state<string | null>(null);
  onMount(() => {
    void kept().then((device) => (here = device?.id ?? null));
  });

  function forget(device: DeviceId): void {
    if (u.send(forgetDevice(device))) u.conn.asking.refresh(QUERIES.devices);
  }
</script>

{#if read.kind === "held"}
  <Look devices={read.value} {here} now={u.now()} local={!u.origin.startsWith("https:")} onForget={forget} />
{:else if read.kind === "unavailable"}
  <Unanswered query={read.query} asked={QUERIES.devices} />
{/if}
