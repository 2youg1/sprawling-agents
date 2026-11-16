<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  const RECEIPT_MS = 400;
</script>

<script lang="ts">
  import { untrack } from "svelte";

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { Address } from "../../wire";

  interface Props {
    readonly room: Address | null;
    readonly sent: number;
  }

  const { room, sent }: Props = $props();

  const lang = ui().lang;

  let handed = $state(false);
  let answered = untrack(() => sent);
  $effect(() => {
    if (sent === answered) return;
    answered = sent;
    handed = true;
    const receipt = setTimeout(() => {
      handed = false;
    }, RECEIPT_MS);
    return () => {
      clearTimeout(receipt);
    };
  });
</script>

<span role="status" class="sr-only">
  {#if handed && room !== null}{fill(say($lang, "talk_handed"), { room })}{/if}
</span>
