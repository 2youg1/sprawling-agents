<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts">
  // The searchable list, open and closed, stacked as a settings card
  // stacks two of them.
  import { say } from "../../../core/lang";
  import { ui } from "../../../ui";
  import Combobox from "../../parts/combobox.svelte";
  import Case from "../case.svelte";
  import { MODELS } from "../served";

  const { lang } = ui();

  let model = $state<string | null>(null);
  const modelChoices = MODELS.map((each) => ({ value: each.id, label: each.id, note: each.context }));
  const pickModel = (value: string): void => {
    model = value;
  };
</script>

<!-- Stacked as the settings page stacks them: the open list covers the second trigger.
The list is positioned and takes no room, so `pb-output` (the list's own max height) grows the section to hold it. -->
<Case label="combobox · open on click, nothing chosen">
  <div class="flex flex-col gap-base pb-output">
    <Combobox label={say($lang, "setup_main")} placeholder={say($lang, "part_search")} empty={say($lang, "part_no_match")}
      choices={modelChoices} value={model} onPick={pickModel} starts="open" />
    <Combobox label={say($lang, "setup_digest")} placeholder={say($lang, "part_search")} empty={say($lang, "part_no_match")}
      choices={modelChoices} value={model} onPick={pickModel} />
  </div>
</Case>
