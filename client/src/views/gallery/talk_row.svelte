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
  import { FILTER_AFTER, pills } from "../talk/composer";
  import PillView from "../talk/pill.svelte";
  import SettingsRow from "../talk/settings_row.svelte";
  import Case from "./case.svelte";
  import { CHOSEN, MODELS } from "./served";

  const { lang } = ui();
  const ignore = (): void => undefined;
  const HERE = Address.make("hall/mayor");
  const ROOMS = ["hall/mayor", ...Array.from({ length: FILTER_AFTER + 2 }, (_unused, index) => `atlas/room-${String(index)}`)];
  const specs = $derived(
    pills(
      $lang,
      {
        served: MODELS.map((each) => ({ endpoint: "gallery", model: each.id, label: "gallery" })),
        chosen: { endpoint: "gallery", model: CHOSEN.id },
        session: null,
        rooms: ROOMS,
        here: HERE,
        effort: null,
      },
      { model: ignore, workspace: ignore, effort: ignore },
    ),
  );
</script>

<Case label="settings row · after a session starts · a sentence the link did not take">
  <SettingsRow {specs} room={HERE} draws="notice" kept={true} />
</Case>
<Case label="settings row · before a session · a sentence the link did not take">
  <SettingsRow {specs} room={null} draws="everything" kept={true} />
</Case>
<Case label="settings row · the workspace chip open on a long list · filter">
  <div class="flex min-h-[48rem] items-end">
    <PillView spec={specs[1]} starts="open" />
  </div>
</Case>
