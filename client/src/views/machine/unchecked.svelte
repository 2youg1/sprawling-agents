<script lang="ts" module>
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The city has not answered about this machine. Not a grey word: a
// state with the one action that leaves it.

export interface UncheckedProps {
  readonly onRecheck: () => void;
}
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Button from "../parts/button.svelte";

  const { onRecheck }: UncheckedProps = $props();

  const { lang } = ui();
</script>

<div class="flex flex-wrap items-center gap-snug">
  <span class="text-note text-text-faint">{say($lang, "machine_unasked")}</span>
  <Button label={say($lang, "machine_recheck")} tone="secondary" onPress={onRecheck} />
</div>
