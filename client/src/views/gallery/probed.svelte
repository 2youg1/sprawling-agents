<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The probed-model table in the states its seat reaches only after a
  // person has typed: rows ticked with figures and roles stated, and an
  // endpoint that served no list, filled in by hand. The look is drawn
  // from the wiring's own value over a state written here, which
  // stays live: a press in the fixture redraws as it does on the page.
  import { ui } from "../../ui";
  import { freshTable, lookOf } from "../setup/model_table";
  import Look from "../setup/model_table.look.svelte";
  import Case from "./case.svelte";
  import { PROBED } from "./served";

  const { lang } = ui();

  const worked = $state(freshTable());
  worked.textOnly = false;
  for (const row of PROBED) worked.ticked[row.id] = true;
  const first = PROBED[0]?.id;
  if (first !== undefined) {
    worked.output[first] = "32768";
    worked.input[first] = "text_image";
    worked.role[first] = "main";
  }

  const typed = $state(freshTable());
  typed.manual = "house/writer, house/reader";
  typed.ticked["house/writer"] = true;

  const workedLook = $derived(lookOf(worked, PROBED, $lang));
  const typedLook = $derived(lookOf(typed, [], $lang));
</script>

<Case label="models · every row ticked, one with its figures and role stated">
  <Look {...workedLook} />
</Case>

<Case label="models · no list served, two ids typed by hand">
  <Look {...typedLook} />
</Case>
