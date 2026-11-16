<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // One hint, in the two places it has to survive.
  //
  // `parts/tip.svelte` places a hint twice over: against the anchor
  // where the engine implements anchor positioning, and against the
  // wrapper where it does not. Until now neither branch had a fixture,
  // which `client/Spec.lean` §2 records under what nobody has verified - a
  // hint that lands outside the window, or one clipped away by the box
  // it sits in, is invisible in exactly the way a person cannot report.
  //
  // **These two lead the page because where they sit is what they
  // test.** A hint opens above its control and flips below when the
  // window has no room above; a fixture halfway down a long page
  // always has room, so the flip never happens and the fallback is
  // never seen. The first one sits as near the top of the window as a
  // fixture on this route can, and its words are long enough to be
  // taller than the strip above it.
  //
  // **The second one is inside a box that scrolls.** Anchored, the
  // hint is `fixed` and leaves that box; placed against the wrapper it
  // is `absolute` and the box clips it. The two readings are visibly
  // different, which is the point: one engine must not quietly swallow
  // what the other shows.
  //
  // A hint is `display: none` until a pointer or the keyboard wants
  // it, so `xtask render` measures no box for either today. What these
  // two give the gate is the page in the state a person meets, ready
  // for the first pass that presses a key (client/Spec.lean §7-10).

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";
  import Case from "./case.svelte";

  const { lang } = ui();
</script>

<Case label="tip · near the top of the window, where a hint has to flip below">
  <Button label={say($lang, "part_save")} tone="primary" why={say($lang, "gallery_tip_tall")} />
</Case>

<Case label="tip · inside a box that scrolls, which clips or is escaped">
  <div class="h-output overflow-auto rounded-card border border-edge-panel p-base">
    <Button label={say($lang, "part_save")} tone="primary" why={say($lang, "part_why_halted")} />
    <!-- A box only scrolls when it holds more than it shows. This is
    that surplus and carries nothing else, so the control above it
    stays at the top edge where its hint has to reach past the
    border. -->
    <div class="h-output" aria-hidden="true"></div>
  </div>
</Case>
