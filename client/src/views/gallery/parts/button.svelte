<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The button in each state it can be in: the four tones, loading, and
  // refused with the reason it gives.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Button from "../../parts/button.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();
</script>

<Case label="button · four tones">
  <div class="flex flex-wrap items-center gap-snug">
    <Button label={say($lang, "part_save")} tone="primary" />
    <Button label={say($lang, "part_cancel")} tone="secondary" />
    <Button label={say($lang, "dismiss")} tone="quiet" />
    <Button label={say($lang, "part_delete")} tone="destructive" />
  </div>
</Case>

<Case label="button · loading and refused">
  <div class="flex flex-wrap items-center gap-snug">
    <Button label={say($lang, "part_save")} tone="primary" loading />
    <Button label={say($lang, "part_save")} tone="primary" why={say($lang, "part_why_halted")} />
  </div>
</Case>
