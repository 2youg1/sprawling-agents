<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  import { say } from "../../../core/lang";
  import { LABELS } from "../../../core/keys";
  import { ui } from "../../../ui";
  import { Kbd } from "../../parts/kbd.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();
</script>

<!-- eslint-disable @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call -->
<Case label="kbd · a chord, a line move and a first letter at a row's end">
  <ul class="flex flex-col">
    <li class="flex items-center justify-between gap-base py-tight text-label">
      <span class="text-text-quiet">{say($lang, LABELS.palette)}</span>
      {@render Kbd({ action: "palette" })}
    </li>
    <li class="flex items-center justify-between gap-base py-tight text-label">
      <span class="text-text-quiet">{say($lang, "part_save")}</span>
      {@render Kbd({ move: "line.next" })}
    </li>
    <li class="flex items-center justify-between gap-base py-tight text-label">
      <span class="text-text-quiet">{say($lang, "part_delete")}</span>
      {@render Kbd({ initial: "d" })}
    </li>
  </ul>
</Case>
