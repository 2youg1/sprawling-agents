<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- The settings row before a session, and the model picker in each of
its states: at rest, opened from each segment of its token, on a model
with no thinking control, on a city whose models have none, and on an
aggregator that serves more models than the city has providers. Each
open picker stands in a box as tall as the page's foot gives it. -->
<script lang="ts">
  import { ui } from "../../ui";
  import { Address } from "../../wire";
  import Tier from "../setup/tier.svelte";
  import { workspacePill } from "../talk/composer";
  import SettingsRow from "../talk/settings_row.svelte";
  import Case from "./case.svelte";
  import { AT_DEEPSEEK, AT_OLLAMA, AT_OPENROUTER, MANY_MODELS, MANY_PROVIDERS, ONE_PROVIDER, pickerFacts } from "./picked";

  const { lang } = ui();
  const ignore = (): void => undefined;
  const workspace = $derived(workspacePill($lang, { rooms: ["hall/mayor"], here: Address.make("hall/mayor") }, ignore));
  const served = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, null);
  const stated = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, "xhigh");
  const local = pickerFacts(MANY_PROVIDERS, AT_OLLAMA, null);
  const alone = pickerFacts(ONE_PROVIDER, AT_OLLAMA, null);
  const aggregated = pickerFacts(MANY_MODELS, AT_OPENROUTER, "medium");
  const none = pickerFacts([], undefined, null);
</script>

<Case label="settings row · before a session · workspace, picker and permissions">
  <SettingsRow {workspace} picker={served} room={null} draws="everything" kept={false} />
</Case>
<Case label="settings row · no offered model · picker hidden">
  <SettingsRow {workspace} picker={none} room={null} draws="everything" kept={false} />
</Case>
<Case label="settings row · the run policy's menu open">
  <div class="flex min-h-[14rem] flex-col justify-end">
    <SettingsRow {workspace} picker={served} room={null} draws="everything" kept={false} menu="open" />
  </div>
</Case>
<Case label="picker · opened from the model · models first, providers of the chosen one">
  <div class="flex min-h-[46rem] flex-col justify-end">
    <SettingsRow {workspace} picker={served} room={null} draws="everything" kept={false} menu="model" />
  </div>
</Case>
<Case label="picker · opened from the provider · a stated level this provider does not offer">
  <div class="flex min-h-[46rem] flex-col justify-end">
    <SettingsRow {workspace} picker={stated} room={null} draws="everything" kept={false} menu="provider" />
  </div>
</Case>
<Case label="picker · opened from the level">
  <div class="flex min-h-[46rem] flex-col justify-end">
    <SettingsRow {workspace} picker={served} room={null} draws="everything" kept={false} menu="level" />
  </div>
</Case>
<Case label="picker · a model with no thinking control · the band's room kept for the others">
  <div class="flex min-h-[46rem] flex-col justify-end">
    <SettingsRow {workspace} picker={local} room={null} draws="everything" kept={false} menu="model" />
  </div>
</Case>
<Case label="picker · one provider, no thinking control anywhere · no band, no room for one">
  <div class="flex min-h-[30rem] flex-col justify-end">
    <SettingsRow {workspace} picker={alone} room={null} draws="everything" kept={false} menu="model" />
  </div>
</Case>
<Case label="picker · more models than providers · providers first, five models and more">
  <div class="flex min-h-[50rem] flex-col justify-end">
    <SettingsRow {workspace} picker={aggregated} room={null} draws="everything" kept={false} menu="model" />
  </div>
</Case>
<Case label="settings row · after a session starts · no controls">
  <SettingsRow {workspace} picker={served} room={null} draws="notice" kept={false} />
</Case>
<Case label="appearance · the tier card" width={390}>
  <Tier />
</Case>
