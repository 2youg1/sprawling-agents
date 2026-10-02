<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts" module>
  // What the person chooses before a session begins and how the page is
  // laid out: the composer's settings row with the write limit beside
  // the mode, the same row with the run policy's menu open, and the
  // appearance group's tier card. The row reads the page's own run
  // policy, so it is drawn at the posture a page opens with.
  import type { Pill } from "../talk/composer";

  function pill(label: string, value: string): Pill {
    return { label, placeholder: label, choices: [{ value, label: value }], value, pick: () => undefined };
  }

  const SPECS = [pill("model", "m-local"), pill("workspace", "hall/mayor"), pill("effort", "high"), pill("mode", "work")] as const;
</script>

<script lang="ts">
  import Tier from "../setup/tier.svelte";
  import SettingsRow from "../talk/settings_row.svelte";
  import Case from "./case.svelte";
</script>

<Case label="settings row · before a session · the write limit beside the mode">
  <SettingsRow specs={SPECS} room={null} draws="everything" kept={false} />
</Case>
<Case label="settings row · the run policy's menu open">
  <div class="flex min-h-[14rem] flex-col justify-end">
    <SettingsRow specs={SPECS} room={null} draws="everything" kept={false} menu="open" />
  </div>
</Case>
<Case label="appearance · the tier card" width={390}>
  <Tier />
</Case>
