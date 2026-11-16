<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Tip from "../../parts/tip.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();
</script>

<Case label="tip · a hint on a control named by its own text">
  <Tip text={say($lang, "part_why_halted")}>
    {#snippet children(hint: string)}
      <button
        type="button"
        class="rounded-control px-base py-tight text-label text-text-quiet hover:bg-raised"
        aria-describedby={hint}
      >
        {say($lang, "part_save")}
      </button>
    {/snippet}
  </Tip>
</Case>
