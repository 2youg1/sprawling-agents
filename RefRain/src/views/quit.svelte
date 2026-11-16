<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  
  
  import { quitAsked } from "../core/quitting";
  import type { CloseMode } from "../wire";
  import { ui } from "../ui";
  import { quitOf } from "./quit";
  import QuitSheet from "./quit_sheet.svelte";

  const u = ui();
  const belief = u.conn.belief;

  let open = $state(false);
  
  let seen = $quitAsked;
  $effect(() => {
    const asks = $quitAsked;
    if (asks === seen) return;
    seen = asks;
    open = true;
  });

  const look = $derived(quitOf({ runs: $belief.live.length, here: u.origin }));

  function close(mode: CloseMode): void {
    open = false;
    u.conn.closeCity(mode);
  }
</script>

<QuitSheet
  {look}
  seat="modal"
  {open}
  onCancel={() => {
    open = false;
  }}
  onClose={close}
/>
