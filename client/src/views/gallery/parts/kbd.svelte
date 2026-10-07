<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The three things a key mark draws beside a row: a shell action's
  // chord, the first key of a line move, and the first letter a row is
  // reached by. The sheet that lists every chord is the specimen under
  // `keys ·` on the screens fold.
  import { say } from "../../../core/lang";
  import { LABELS } from "../../../core/keys";
  import { ui } from "../../../ui";
  import { Kbd } from "../../parts/kbd.svelte";
  import Case from "../case.svelte";

  const { lang } = ui();
</script>

<Case label="kbd · a chord, a line move and a first letter at a row's end">
  <ul class="flex flex-col">
    <li class="flex items-center justify-between gap-base py-tight text-label">
      <span class="text-text-quiet">{say($lang, LABELS["decide.yes"])}</span>
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (a snippet call is the render itself; svelte-check types this imported snippet fine, and typescript-eslint does not resolve exports of another .svelte module) -->
      {@render Kbd({ action: "decide.yes" })}
    </li>
    <li class="flex items-center justify-between gap-base py-tight text-label">
      <span class="text-text-quiet">{say($lang, "part_save")}</span>
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (as above) -->
      {@render Kbd({ move: "line.next" })}
    </li>
    <li class="flex items-center justify-between gap-base py-tight text-label">
      <span class="text-text-quiet">{say($lang, "part_delete")}</span>
      <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression, @typescript-eslint/no-unsafe-call (as above) -->
      {@render Kbd({ initial: "d" })}
    </li>
  </ul>
</Case>
