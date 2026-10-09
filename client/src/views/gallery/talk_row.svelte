<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The settings row's states that `g2.svelte` and `talking.svelte` do
not draw: the notice of a sentence the link did not take, on its own and
beside the entries, and the workspace chip's menu long enough to carry
a filter, in a box tall enough to hold the whole menu with its filter. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { Address } from "../../wire";
  import { FILTER_AFTER, workspacePill } from "../talk/composer";
  import PillView from "../talk/pill.svelte";
  import SettingsRow from "../talk/settings_row.svelte";
  import Case from "./case.svelte";
  import { AT_DEEPSEEK, MANY_PROVIDERS, pickerFacts } from "./picked";

  const { lang } = ui();
  const ignore = (): void => undefined;
  const HERE = Address.make("hall/mayor");
  const ROOMS = ["hall/mayor", ...Array.from({ length: FILTER_AFTER + 2 }, (_unused, index) => `atlas/room-${String(index)}`)];
  const workspace = $derived(workspacePill($lang, { rooms: ROOMS, here: HERE }, ignore));
  const picker = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, null);
</script>

<Case label="settings row · after a session starts · a sentence the link did not take">
  <SettingsRow {workspace} {picker} room={HERE} draws="notice" kept={true} />
</Case>
<Case label="settings row · before a session · a sentence the link did not take">
  <SettingsRow {workspace} {picker} room={null} draws="everything" kept={true} />
</Case>
<Case label="settings row · the workspace chip open on a long list · filter">
  <div class="flex min-h-[48rem] items-end">
    <PillView spec={workspace} starts="open" />
  </div>
</Case>
