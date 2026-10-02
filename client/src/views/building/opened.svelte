<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The building page's right side: the file of this building that the
  // right-side state has in front (client/Spec.lean §4-50, client D27). It reads
  // the same state the conversation's right side reads, so a file a
  // change row opened here is still open in the conversation, and
  // closing it here closes it there. The band on top is the one surface
  // lifted to `chrome` (7A-4); the document stays on the page.
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { closeRight } from "../inspect/open.svelte";
  import type { DocumentItem } from "../inspect/open.svelte";
  import Button from "../parts/button.svelte";
  import RefRain from "../refrain/refrain.svelte";

  interface Props {
    readonly item: DocumentItem;
  }

  const { item }: Props = $props();

  const lang = ui().lang;
</script>

<aside
  class="col-[8/12] row-[1/3] flex min-w-0 flex-col overflow-hidden rounded-panel border border-edge narrow:col-span-full narrow:row-auto"
  aria-label={item.path}
>
  <!-- The band carries the one control the side owns; the file's path
       is the document's own head, so it is not written here again. -->
  <div class="flex h-bar shrink-0 items-center justify-end bg-chrome px-snug">
    <Button label={say($lang, "panel_close")} tone="quiet" onPress={closeRight} />
  </div>
  <div class="min-h-0 min-w-0 px-base py-base">
    <RefRain building={item.building} path={item.path} version={item.version} />
  </div>
</aside>
