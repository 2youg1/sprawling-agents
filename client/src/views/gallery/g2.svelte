<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { EFFORTS } from "../../core/commands";
  import { ui } from "../../ui";
  import Tier from "../setup/tier.svelte";
  import { FILTER_AFTER, pills } from "../talk/composer";
  import SettingsRow from "../talk/settings_row.svelte";
  import Case from "./case.svelte";
  import { CHOSEN, MODELS } from "./served";
  import { Address } from "../../wire";

  const { lang } = ui();
  const ignore = (): void => undefined;
  const specs = $derived(pills($lang, {
    served: MODELS.map((each, index) => ({ endpoint: index === 0 ? "gallery" : "alternate", model: each.id, label: index === 0 ? "gallery" : "alternate" })),
    chosen: { endpoint: "gallery", model: CHOSEN.id },
    session: null,
    rooms: ["hall/mayor"],
    here: Address.make("hall/mayor"),
    effort: EFFORTS[2] ?? null,
  }, { model: ignore, workspace: ignore, effort: ignore }));
  const many = $derived(pills($lang, {
    served: [{ endpoint: "gallery", model: CHOSEN.id, label: "gallery" }, ...Array.from({ length: FILTER_AFTER + 4 }, (_unused, index) => ({ endpoint: "gallery", model: `gallery/model-${String(index)}`, label: "gallery" })), { endpoint: "alternate", model: "alternate/small", label: "alternate" }],
    chosen: { endpoint: "gallery", model: CHOSEN.id }, session: null,
    rooms: ["hall/mayor"], here: Address.make("hall/mayor"), effort: null,
  }, { model: ignore, workspace: ignore, effort: ignore }));
  const empty = $derived(pills($lang, {
    served: [], chosen: undefined, session: null, rooms: ["hall/mayor"],
    here: Address.make("hall/mayor"), effort: null,
  }, { model: ignore, workspace: ignore, effort: ignore }));
</script>

<Case label="settings row · before a session · workspace, model and permissions">
  <SettingsRow {specs} room={null} draws="everything" kept={false} />
</Case>
<Case label="settings row · no offered model · model entry hidden">
  <SettingsRow specs={empty} room={null} draws="everything" kept={false} />
</Case>
<Case label="settings row · the run policy's menu open">
  <div class="flex min-h-[14rem] flex-col justify-end">
    <SettingsRow {specs} room={null} draws="everything" kept={false} menu="open" />
  </div>
</Case>
<Case label="settings row · provider with its model and thinking">
  <div class="flex min-h-[36rem] flex-col justify-end">
    <SettingsRow {specs} room={null} draws="everything" kept={false} menu="model" />
  </div>
</Case>
<Case label="settings row · provider with many models · search">
  <div class="flex min-h-[36rem] flex-col justify-end">
    <SettingsRow specs={many} room={null} draws="everything" kept={false} menu="model" />
  </div>
</Case>
<Case label="settings row · after a session starts · no controls">
  <SettingsRow {specs} room={null} draws="notice" kept={false} />
</Case>
<Case label="appearance · the tier card" width={390}>
  <Tier />
</Case>
