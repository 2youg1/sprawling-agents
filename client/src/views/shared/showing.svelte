<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The results-only switch, one control with three seats: a room, the
  // city and the appearance settings. Every seat writes the same
  // preference, so the choice made on a page is the default the next
  // page opens with, and the settings seat is where it is set on purpose.
  import { say } from "../../core/lang";
  import { SHOWINGS } from "../../core/results";
  import type { Showing } from "../../core/results";
  import { ui } from "../../ui";
  import Segmented from "../parts/segmented.svelte";

  const { prefs, lang } = ui();
  const { held } = prefs;
</script>

<Segmented
  label={say($lang, "showing_label")}
  options={SHOWINGS.map((value) => ({ value, label: say($lang, `showing_${value}`) }))}
  held={$held.showing}
  onPick={(showing: Showing) => {
    prefs.setShowing(showing);
  }}
/>
