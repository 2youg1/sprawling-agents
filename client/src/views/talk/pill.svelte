<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- One picker pill in the composer's mode row: which model answers,
which room hears it, how hard the model thinks. `composer.ts` owns what
the pills offer and what a pick means; this is the shape three of them
are drawn in. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { say } from "../../core/lang";
  import Combobox from "../parts/combobox.svelte";
  import type { Pill } from "./composer";

  interface Props {
    readonly spec: Pill;
  }

  const { spec }: Props = $props();

  const u = ui();
  const { lang } = u;
</script>

<div class="min-w-0 grow basis-[9rem]">
  <Combobox
    label={spec.label}
    placeholder={spec.placeholder}
    empty={say($lang, "part_no_match")}
    choices={spec.choices}
    value={spec.value}
    onPick={spec.pick}
  />
</div>
