<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The question `/quit` asks before the city closes (client/Spec.lean
  // §4-57c). With no run going it is close or cancel; with runs going it
  // is wait for them, stop them now, or cancel, the same three answers
  // the console's `/quit` gives. A page opened through the remote door
  // is told only that closing belongs to this machine, because the
  // relay refuses the verb from a device (`LocalOnly`). The first
  // control is the way out, so the safe answer is under the hand; the
  // box is `quit_sheet.svelte`, which the gallery draws as a specimen.
  import { quitAsked } from "../core/quitting";
  import type { CloseMode } from "../wire";
  import { ui } from "../ui";
  import { quitOf } from "./quit";
  import QuitSheet from "./quit_sheet.svelte";

  const u = ui();
  const belief = u.conn.belief;

  let open = $state(false);
  // The count the page was opened with is no ask: only a press after it is.
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
