<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The settings row's states that `g2.svelte` and `talking.svelte` do
not draw: the notice of a sentence the link did not take, on its own and
beside the entries. -->
<script lang="ts">
  import { Address } from "../../wire";
  import SettingsRow from "../talk/settings_row.svelte";
  import Case from "./case.svelte";
  import { AT_DEEPSEEK, MANY_PROVIDERS, pickerFacts } from "./picked";

  const HERE = Address.make("hall/mayor");
  const picker = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, null);
</script>

<Case label="settings row · after a session starts · a sentence the link did not take">
  <SettingsRow {picker} room={HERE} draws="notice" kept={true} />
</Case>
<Case label="settings row · before a session · a sentence the link did not take">
  <SettingsRow {picker} room={null} draws="everything" kept={true} />
</Case>
